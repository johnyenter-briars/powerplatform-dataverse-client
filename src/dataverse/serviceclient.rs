use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use log::debug;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde_json::Map;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::LogLevel;
use crate::auth::config::AuthConfig;
use crate::auth::connectionstring::{
    parse_connection_string_auth_config, parse_connection_string_url,
};
use crate::auth::credentials::{TokenExchange, refresh_device_code_token};
use crate::auth::token::{
    CachedToken, fetch_token_for_config, is_expiring_soon, load_cached_token,
    resolve_token_cache_file_path, save_cached_token,
};
use crate::dataverse::entity::Entity;
use crate::dataverse::entity::Value::Int;
use crate::dataverse::entityattribute::EntityAttribute;
use crate::dataverse::entitydefinition::EntityDefinition;
use crate::dataverse::entityrelationship::EntityRelationship;
use crate::dataverse::fetchxml::{apply_paging, ensure_aggregate_page_size, fetch_tag_has_attr};
use crate::dataverse::parse::{
    extract_paging_cookie, parse_entities_from_response, parse_more_records,
    parse_record_count_from_response,
};
use crate::dataverse::requestparameters::RequestParameters;

const ROW_NUMBER_ATTRIBUTE: &str = "__rownum";
const AGGREGATE_PAGE_SIZE: i32 = 5000;

/// OData list wrapper returned by Dataverse metadata endpoints.
#[derive(Debug, serde::Deserialize)]
struct ODataList<T> {
    value: Vec<T>,
}

#[derive(Debug, serde::Deserialize)]
struct EntityRelationshipDirectional {
    #[serde(rename = "SchemaName")]
    schema_name: String,
    #[serde(rename = "ReferencedEntity")]
    referenced_entity: Option<String>,
    #[serde(rename = "ReferencedAttribute")]
    referenced_attribute: Option<String>,
    #[serde(rename = "ReferencingEntity")]
    referencing_entity: Option<String>,
    #[serde(rename = "ReferencingAttribute")]
    referencing_attribute: Option<String>,
    #[serde(rename = "IsCustomRelationship")]
    is_custom_relationship: Option<bool>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

#[derive(Debug, serde::Deserialize)]
struct EntityRelationshipManyToMany {
    #[serde(rename = "SchemaName")]
    schema_name: String,
    #[serde(rename = "Entity1LogicalName")]
    entity1_logical_name: Option<String>,
    #[serde(rename = "Entity2LogicalName")]
    entity2_logical_name: Option<String>,
    #[serde(rename = "IntersectEntityName")]
    intersect_entity_name: Option<String>,
    #[serde(rename = "IsCustomRelationship")]
    is_custom_relationship: Option<bool>,
    #[serde(flatten)]
    extra: Map<String, Value>,
}

/// HTTP client for Dataverse Web API operations.
pub struct ServiceClient {
    client: Client,
    auth: AuthConfig,
    base_url: std::string::String,
    token_cache_path: PathBuf,
    token: Mutex<CachedToken>,
    entity_definitions_cache: Mutex<Option<Vec<EntityDefinition>>>,
    entity_attributes_cache: Mutex<HashMap<String, Vec<EntityAttribute>>>,
    log_level: LogLevel,
}

impl ServiceClient {
    /// Create a new client from a Dataverse connection string.
    pub async fn new(connection_string: &str, log_level: LogLevel) -> Result<Self, String> {
        let base_url = parse_connection_string_url(connection_string)?;
        let auth = parse_connection_string_auth_config(connection_string)?;
        Self::new_internal(auth, base_url, log_level).await
    }

    /// Create a new client from explicit authentication configuration.
    pub async fn new_with_auth(auth: AuthConfig, log_level: LogLevel) -> Result<Self, String> {
        let base_url = auth.dataverse_url().to_string();
        Self::new_internal(auth, base_url, log_level).await
    }

    async fn new_internal(
        auth: AuthConfig,
        base_url: String,
        log_level: LogLevel,
    ) -> Result<Self, String> {
        let token_cache_path = resolve_token_cache_file_path(&auth)?;

        let token = if let Some(cached) = load_cached_token(&token_cache_path)? {
            if !cached.access_token.trim().is_empty() && !is_expiring_soon(cached.expires_at) {
                cached
            } else {
                let refreshed = fetch_token_for_config(&auth).await?;
                save_cached_token(&token_cache_path, &refreshed)?;
                refreshed
            }
        } else {
            let fetched = fetch_token_for_config(&auth).await?;
            save_cached_token(&token_cache_path, &fetched)?;
            fetched
        };

        Ok(Self {
            client: Client::new(),
            auth,
            base_url,
            token_cache_path,
            token: Mutex::new(token),
            entity_definitions_cache: Mutex::new(None),
            entity_attributes_cache: Mutex::new(HashMap::new()),
            log_level,
        })
    }

    /// Return the current token expiry as a UTC datetime.
    pub async fn token_expires_at(&self) -> Option<DateTime<Utc>> {
        let expires_at = self.token.lock().await.expires_at?;
        DateTime::<Utc>::from_timestamp(expires_at as i64, 0)
    }

    /// Retrieve multiple records by FetchXML, handling paging when needed.
    pub async fn retrieve_multiple_fetchxml(
        &self,
        entity: &str,
        fetchxml: &str,
    ) -> Result<Vec<Entity>, std::string::String> {
        let primary_id_attribute = self.resolve_primary_id_attribute(entity).await?;
        let attribute_map = self.entity_attribute_map(entity).await?;
        if fetch_tag_has_attr(fetchxml, "top")? {
            return self
                .retrieve_multiple_fetchxml_single(
                    entity,
                    fetchxml,
                    primary_id_attribute.as_deref(),
                    Some(&attribute_map),
                )
                .await;
        }

        let mut page = 1;
        let mut paging_cookie: Option<std::string::String> = None;
        let mut entities: Vec<Entity> = vec![];

        loop {
            let fetch_with_paging = apply_paging(
                &ensure_aggregate_page_size(fetchxml, AGGREGATE_PAGE_SIZE)?,
                page,
                paging_cookie.as_deref(),
            )?;

            if self.log_level.includes_debug() {
                debug!("Fetch page: {}", page);
                debug!("FetchXML: {}", fetch_with_paging);
            }

            let mut url = format!("{}/api/data/v9.2/{}", self.base_url, entity);
            url.push_str("?fetchXml=");
            url.push_str(&urlencoding::encode(&fetch_with_paging));

            if self.log_level.includes_debug() {
                debug!("Url: {:?}", url);
            }

            let access_token = self.get_access_token().await?;
            let resp = self
                .client
                .get(&url)
                .bearer_auth(&access_token)
                .header("Accept", "application/json")
                .header(
                    "Prefer",
                    "odata.include-annotations=\"Microsoft.Dynamics.CRM.fetchxmlpagingcookie,Microsoft.Dynamics.CRM.morerecords,Microsoft.Dynamics.CRM.lookuplogicalname,OData.Community.Display.V1.FormattedValue\"",
                )
                .send()
                .await
                .map_err(|e| format!("Request failed: {e}"))?;

            let status = resp.status();

            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Dataverse API error ({}): {}", status, body));
            }

            let json: Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse JSON: {e}"))?;

            let mut page_entities = parse_entities_from_response(
                &json,
                entity,
                primary_id_attribute.as_deref(),
                Some(&attribute_map),
            )?;
            let start_index = entities.len();
            for (offset, entity) in page_entities.iter_mut().enumerate() {
                let row_number = (start_index + offset + 1) as i64;
                entity
                    .attributes
                    .insert(ROW_NUMBER_ATTRIBUTE.to_string(), Int(row_number));
            }
            entities.extend(page_entities);

            let more_records = parse_more_records(&json);
            if !more_records {
                break;
            }

            paging_cookie = extract_paging_cookie(&json);
            page += 1;
        }

        Ok(entities)
    }

    /// Count records for a FetchXML query without retrieving all data.
    pub async fn retrieve_multiple_fetchxml_count(
        &self,
        entity: &str,
        fetchxml: &str,
    ) -> Result<usize, std::string::String> {
        if fetch_tag_has_attr(fetchxml, "top")? {
            let resp = self
                .retrieve_multiple_fetchxml_single(entity, fetchxml, None, None)
                .await?;
            return Ok(resp.len());
        }

        let mut page = 1;
        let mut paging_cookie: Option<std::string::String> = None;
        let mut total = 0usize;

        loop {
            let fetch_with_paging = apply_paging(
                &ensure_aggregate_page_size(fetchxml, AGGREGATE_PAGE_SIZE)?,
                page,
                paging_cookie.as_deref(),
            )?;

            if self.log_level.includes_debug() {
                debug!("Fetch page: {}", page);
                debug!("FetchXML: {}", fetch_with_paging);
            }

            let mut url = format!("{}/api/data/v9.2/{}", self.base_url, entity);
            url.push_str("?fetchXml=");
            url.push_str(&urlencoding::encode(&fetch_with_paging));

            if self.log_level.includes_debug() {
                debug!("Url: {:?}", url);
            }

            let access_token = self.get_access_token().await?;
            let resp = self
                .client
                .get(&url)
                .bearer_auth(&access_token)
                .header("Accept", "application/json")
                .header(
                    "Prefer",
                    "odata.include-annotations=\"Microsoft.Dynamics.CRM.fetchxmlpagingcookie,Microsoft.Dynamics.CRM.morerecords,Microsoft.Dynamics.CRM.lookuplogicalname,OData.Community.Display.V1.FormattedValue\"",
                )
                .send()
                .await
                .map_err(|e| format!("Request failed: {e}"))?;

            let status = resp.status();

            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(format!("Dataverse API error ({}): {}", status, body));
            }

            let json: Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse JSON: {e}"))?;

            total += parse_record_count_from_response(&json)?;

            let more_records = parse_more_records(&json);
            if !more_records {
                break;
            }

            paging_cookie = extract_paging_cookie(&json);
            page += 1;
        }

        Ok(total)
    }

    /// Retrieve a single page of FetchXML results.
    async fn retrieve_multiple_fetchxml_single(
        &self,
        entity: &str,
        fetchxml: &str,
        primary_id_attribute: Option<&str>,
        entity_attributes: Option<&HashMap<String, EntityAttribute>>,
    ) -> Result<Vec<Entity>, std::string::String> {
        if self.log_level.includes_debug() {
            debug!("FetchXML: {}", fetchxml);
        }

        let mut url = format!("{}/api/data/v9.2/{}", self.base_url, entity);
        url.push_str("?fetchXml=");
        url.push_str(&urlencoding::encode(fetchxml));

        if self.log_level.includes_debug() {
            debug!("Url: {:?}", url);
        }

        let access_token = self.get_access_token().await?;
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&access_token)
            .header("Accept", "application/json")
            .header(
                "Prefer",
                "odata.include-annotations=\"Microsoft.Dynamics.CRM.fetchxmlpagingcookie,Microsoft.Dynamics.CRM.morerecords,Microsoft.Dynamics.CRM.lookuplogicalname,OData.Community.Display.V1.FormattedValue\"",
            )
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Dataverse API error ({}): {}", status, body));
        }

        let json: Value = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))?;

        parse_entities_from_response(&json, entity, primary_id_attribute, entity_attributes)
    }

    /// List all entity definitions.
    pub async fn list_entity_definitions(
        &self,
    ) -> Result<Vec<EntityDefinition>, std::string::String> {
        {
            let cache = self.entity_definitions_cache.lock().await;
            if let Some(value) = &*cache {
                return Ok(value.clone());
            }
        }

        let url = format!(
            "{}/api/data/v9.2/EntityDefinitions?$select=LogicalName,SchemaName,DisplayName,EntitySetName,IsCustomEntity,IsActivity,PrimaryIdAttribute",
            self.base_url
        );

        let access_token = self.get_access_token().await?;
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&access_token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Dataverse API error ({}): {}", status, body));
        }

        let parsed: ODataList<EntityDefinition> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))?;

        let value = parsed.value;
        let mut cache = self.entity_definitions_cache.lock().await;
        *cache = Some(value.clone());

        Ok(value)
    }

    /// List entity attributes for a given logical name.
    pub async fn list_entity_attributes(
        &self,
        logical_name: &str,
    ) -> Result<Vec<EntityAttribute>, std::string::String> {
        {
            let cache = self.entity_attributes_cache.lock().await;
            if let Some(value) = cache.get(&normalize_entity_name(logical_name)) {
                return Ok(value.clone());
            }
        }

        let logical = logical_name.replace('\'', "''");
        let url = format!(
            "{}/api/data/v9.2/EntityDefinitions(LogicalName='{}')/Attributes?$select=LogicalName,SchemaName,AttributeType,AttributeTypeName,IsCustomAttribute,IsValidODataAttribute,IsValidForRead,IsValidForUpdate&$filter=IsValidODataAttribute eq true and IsValidForRead eq true",
            self.base_url, logical
        );

        let access_token = self.get_access_token().await?;
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&access_token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Dataverse API error ({}): {}", status, body));
        }

        let parsed: ODataList<EntityAttribute> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))?;

        let value = parsed.value;
        let mut cache = self.entity_attributes_cache.lock().await;
        cache.insert(normalize_entity_name(logical_name), value.clone());

        Ok(value)
    }

    /// List entity relationships for a given logical name.
    pub async fn list_entity_relationships(
        &self,
        logical_name: &str,
    ) -> Result<Vec<EntityRelationship>, std::string::String> {
        let logical = logical_name.replace('\'', "''");
        let many_to_one = self
            .list_metadata_collection::<EntityRelationshipDirectional>(&format!(
                "EntityDefinitions(LogicalName='{}')/ManyToOneRelationships?$select=SchemaName,ReferencedEntity,ReferencedAttribute,ReferencingEntity,ReferencingAttribute,IsCustomRelationship",
                logical
            ))
            .await?
            .into_iter()
            .map(|relationship| EntityRelationship {
                schema_name: relationship.schema_name,
                relationship_type: "ManyToOne".to_string(),
                referenced_entity: relationship.referenced_entity,
                referenced_attribute: relationship.referenced_attribute,
                referencing_entity: relationship.referencing_entity,
                referencing_attribute: relationship.referencing_attribute,
                intersect_entity_name: None,
                is_custom_relationship: relationship.is_custom_relationship,
                extra: relationship.extra.into_iter().collect(),
            });

        let one_to_many = self
            .list_metadata_collection::<EntityRelationshipDirectional>(&format!(
                "EntityDefinitions(LogicalName='{}')/OneToManyRelationships?$select=SchemaName,ReferencedEntity,ReferencedAttribute,ReferencingEntity,ReferencingAttribute,IsCustomRelationship",
                logical
            ))
            .await?
            .into_iter()
            .map(|relationship| EntityRelationship {
                schema_name: relationship.schema_name,
                relationship_type: "OneToMany".to_string(),
                referenced_entity: relationship.referenced_entity,
                referenced_attribute: relationship.referenced_attribute,
                referencing_entity: relationship.referencing_entity,
                referencing_attribute: relationship.referencing_attribute,
                intersect_entity_name: None,
                is_custom_relationship: relationship.is_custom_relationship,
                extra: relationship.extra.into_iter().collect(),
            });

        let many_to_many = self
            .list_metadata_collection::<EntityRelationshipManyToMany>(&format!(
                "EntityDefinitions(LogicalName='{}')/ManyToManyRelationships?$select=SchemaName,Entity1LogicalName,Entity2LogicalName,IntersectEntityName,IsCustomRelationship",
                logical
            ))
            .await?
            .into_iter()
            .map(|relationship| EntityRelationship {
                schema_name: relationship.schema_name,
                relationship_type: "ManyToMany".to_string(),
                referenced_entity: relationship.entity1_logical_name,
                referenced_attribute: None,
                referencing_entity: relationship.entity2_logical_name,
                referencing_attribute: None,
                intersect_entity_name: relationship.intersect_entity_name,
                is_custom_relationship: relationship.is_custom_relationship,
                extra: relationship.extra.into_iter().collect(),
            });

        Ok(many_to_one.chain(one_to_many).chain(many_to_many).collect())
    }

    /// Update a single entity record by ID.
    pub async fn update_entity(
        &self,
        entity_set: &str,
        id: &str,
        attributes: &HashMap<std::string::String, Value>,
    ) -> Result<(), std::string::String> {
        self.update_entity_with_options(entity_set, id, attributes, &RequestParameters::default())
            .await
    }

    /// Update a single entity record by ID with Dataverse request parameters.
    pub async fn update_entity_with_options(
        &self,
        entity_set: &str,
        id: &str,
        attributes: &HashMap<std::string::String, Value>,
        options: &RequestParameters,
    ) -> Result<(), std::string::String> {
        let trimmed = id.trim_matches(|ch| ch == '{' || ch == '}');
        let url = format!(
            "{}/api/data/v9.2/{}({})",
            self.base_url, entity_set, trimmed
        );

        let access_token = self.get_access_token().await?;
        let request = self
            .client
            .patch(&url)
            .bearer_auth(&access_token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&attributes);

        let resp = options
            .apply(request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Dataverse API error ({}): {}", status, body));
        }

        Ok(())
    }

    /// Delete a single entity record by ID.
    pub async fn delete_entity(
        &self,
        entity_set: &str,
        id: &str,
    ) -> Result<(), std::string::String> {
        self.delete_entity_with_options(entity_set, id, &RequestParameters::default())
            .await
    }

    /// Delete a single entity record by ID with Dataverse request parameters.
    pub async fn delete_entity_with_options(
        &self,
        entity_set: &str,
        id: &str,
        options: &RequestParameters,
    ) -> Result<(), std::string::String> {
        let trimmed = id.trim_matches(|ch| ch == '{' || ch == '}');
        let url = format!(
            "{}/api/data/v9.2/{}({})",
            self.base_url, entity_set, trimmed
        );

        let access_token = self.get_access_token().await?;
        let request = self
            .client
            .delete(&url)
            .bearer_auth(&access_token)
            .header("Accept", "application/json");

        let resp = options
            .apply(request)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Dataverse API error ({}): {}", status, body));
        }

        Ok(())
    }

    async fn get_access_token(&self) -> Result<String, String> {
        let mut token = self.token.lock().await;
        if !token.access_token.trim().is_empty() && !is_expiring_soon(token.expires_at) {
            return Ok(token.access_token.clone());
        }

        let refreshed = match &self.auth {
            AuthConfig::ClientCredentials { .. } => {
                println!("Refreshing access token before request using client credentials...");
                fetch_token_for_config(&self.auth).await?
            }
            AuthConfig::DeviceCode {
                client_id,
                dataverse_url,
                tenant_id,
                ..
            } => {
                println!("Refreshing access token before request using device code...");
                let refresh_token = token.refresh_token.clone().ok_or(
                    "Device code token cannot refresh without a refresh token".to_string(),
                )?;
                let scope = format!(
                    "{}/user_impersonation offline_access openid profile",
                    dataverse_url
                );
                let token: TokenExchange =
                    refresh_device_code_token(client_id, tenant_id, &scope, &refresh_token).await?;

                CachedToken {
                    access_token: token.access_token,
                    refresh_token: Some(token.refresh_token),
                    expires_at: Some(token.expires_at),
                }
            }
        };

        save_cached_token(&self.token_cache_path, &refreshed)?;
        let access_token = refreshed.access_token.clone();
        *token = refreshed;
        Ok(access_token)
    }

    async fn list_metadata_collection<T>(&self, path: &str) -> Result<Vec<T>, String>
    where
        T: DeserializeOwned,
    {
        let url = format!("{}/api/data/v9.2/{}", self.base_url, path);

        let access_token = self.get_access_token().await?;
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&access_token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();

        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Dataverse API error ({}): {}", status, body));
        }

        let parsed: ODataList<T> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON: {e}"))?;

        Ok(parsed.value)
    }

    async fn resolve_primary_id_attribute(
        &self,
        entity_set: &str,
    ) -> Result<Option<String>, String> {
        let definitions = self.list_entity_definitions().await?;
        let target = normalize_entity_name(entity_set);

        Ok(definitions
            .into_iter()
            .find(|definition| {
                normalize_entity_name(&definition.entity_set_name) == target
                    || normalize_entity_name(&definition.logical_name) == target
                    || normalize_entity_name(&definition.schema_name) == target
            })
            .and_then(|definition| definition.primary_id_attribute))
    }

    async fn resolve_entity_logical_name(&self, entity_name: &str) -> Result<String, String> {
        let definitions = self.list_entity_definitions().await?;
        let target = normalize_entity_name(entity_name);

        definitions
            .into_iter()
            .find(|definition| {
                normalize_entity_name(&definition.entity_set_name) == target
                    || normalize_entity_name(&definition.logical_name) == target
                    || normalize_entity_name(&definition.schema_name) == target
            })
            .map(|definition| definition.logical_name)
            .ok_or_else(|| format!("Entity metadata not found for '{}'", entity_name))
    }

    async fn entity_attribute_map(
        &self,
        entity_name: &str,
    ) -> Result<HashMap<String, EntityAttribute>, String> {
        let logical_name = self.resolve_entity_logical_name(entity_name).await?;
        let attributes = self.list_entity_attributes(&logical_name).await?;
        let mut map = HashMap::new();
        for attribute in attributes {
            map.insert(attribute.logical_name.to_ascii_lowercase(), attribute.clone());
            map.entry(attribute.schema_name.to_ascii_lowercase())
                .or_insert(attribute);
        }
        Ok(map)
    }
}

fn normalize_entity_name(value: &str) -> String {
    value
        .trim_matches(|ch| ch == '[' || ch == ']' || ch == '"' || ch == '`')
        .to_ascii_lowercase()
}
