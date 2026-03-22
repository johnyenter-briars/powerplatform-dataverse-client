use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use log::debug;
use reqwest::Client;
use reqwest::header::CONTENT_TYPE;
use serde::de::DeserializeOwned;
use serde_json::Map;
use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

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
use crate::dataverse::batch::{
    ExecuteMultipleRequest, ExecuteMultipleResponse, ExecuteMultipleResponseItem,
    OrganizationRequest, ParsedBatchPart, PreparedBatchItem, PreparedBatchRequest,
    entity_to_write_body, parse_batch_response_parts, parse_fault,
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
const DEFAULT_FETCHXML_PAGE_SIZE: i32 = 5000;

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

    /// Retrieve a single FetchXML response page without automatic paging.
    pub async fn retrieve_multiple_fetchxml(
        &self,
        entity: &str,
        fetchxml: &str,
    ) -> Result<Vec<Entity>, std::string::String> {
        let primary_id_attribute = self.resolve_primary_id_attribute(entity).await?;
        let attribute_map = self.entity_attribute_map(entity).await?;
        self.retrieve_multiple_fetchxml_single(
            entity,
            fetchxml,
            primary_id_attribute.as_deref(),
            Some(&attribute_map),
        )
        .await
    }

    /// Retrieve multiple records by FetchXML, automatically paging until all results are returned.
    pub async fn retrieve_multiple_fetchxml_paging(
        &self,
        entity: &str,
        fetchxml: &str,
    ) -> Result<Vec<Entity>, std::string::String> {
        self.retrieve_multiple_fetchxml_paging_with_progress(entity, fetchxml, |_, _| {}, None)
            .await
    }

    /// Retrieve multiple records by FetchXML, automatically paging until all results are returned.
    /// Uses the provided page size when specified, otherwise defaults to 5000 records per page.
    /// Reports page-level progress as `(page_number, total_records_retrieved_so_far)`.
    pub async fn retrieve_multiple_fetchxml_paging_with_progress<F>(
        &self,
        entity: &str,
        fetchxml: &str,
        mut on_progress: F,
        page_size: Option<i32>,
    ) -> Result<Vec<Entity>, std::string::String>
    where
        F: FnMut(usize, usize),
    {
        let page_size = page_size.unwrap_or(DEFAULT_FETCHXML_PAGE_SIZE);
        let primary_id_attribute = self.resolve_primary_id_attribute(entity).await?;
        let attribute_map = self.entity_attribute_map(entity).await?;
        if fetch_tag_has_attr(fetchxml, "top")? {
            let entities = self
                .retrieve_multiple_fetchxml_single(
                    entity,
                    fetchxml,
                    primary_id_attribute.as_deref(),
                    Some(&attribute_map),
                )
                .await?;
            on_progress(1, entities.len());
            return Ok(entities);
        }

        let mut page = 1;
        let mut paging_cookie: Option<std::string::String> = None;
        let mut entities: Vec<Entity> = vec![];

        loop {
            let fetchxml = ensure_fetch_page_size(fetchxml, page_size)?;
            let fetch_with_paging = apply_paging(
                &ensure_aggregate_page_size(&fetchxml, AGGREGATE_PAGE_SIZE)?,
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
            on_progress(page as usize, entities.len());

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

    /// Create a single entity record and return its ID when available.
    pub async fn create_entity(
        &self,
        entity_set: &str,
        attributes: &HashMap<std::string::String, Value>,
    ) -> Result<Option<Uuid>, std::string::String> {
        self.create_entity_with_options(entity_set, attributes, &RequestParameters::default())
            .await
    }

    /// Create a single entity record with Dataverse request parameters.
    pub async fn create_entity_with_options(
        &self,
        entity_set: &str,
        attributes: &HashMap<std::string::String, Value>,
        options: &RequestParameters,
    ) -> Result<Option<Uuid>, std::string::String> {
        let url = format!("{}/api/data/v9.2/{}", self.base_url, entity_set);

        let access_token = self.get_access_token().await?;
        let request = self
            .client
            .post(&url)
            .bearer_auth(&access_token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(attributes);

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

        Ok(resp
            .headers()
            .get("OData-EntityId")
            .or_else(|| resp.headers().get("Location"))
            .and_then(|value| value.to_str().ok())
            .and_then(parse_uuid_from_uri))
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

    /// Execute multiple create, update, and delete requests using a single Dataverse batch call.
    pub async fn execute_multiple(
        &self,
        request: &ExecuteMultipleRequest,
    ) -> Result<ExecuteMultipleResponse, String> {
        if request.requests.is_empty() {
            return Ok(ExecuteMultipleResponse::default());
        }

        if request.requests.len() > 1000 {
            return Err(format!(
                "ExecuteMultipleRequest contains {} requests, exceeding the Dataverse batch limit of 1000",
                request.requests.len()
            ));
        }

        let entity_set_name_by_logical_name = self.entity_set_name_map().await?;
        let prepared_requests = self
            .prepare_batch_requests(&request.requests, &entity_set_name_by_logical_name)?;
        let boundary = format!("batch_{}", Uuid::new_v4().as_hyphenated());
        let body = self.build_batch_body(&boundary, &prepared_requests);
        let url = format!("{}/api/data/v9.2/$batch", self.base_url);
        let access_token = self.get_access_token().await?;

        let mut http_request = self
            .client
            .post(&url)
            .bearer_auth(&access_token)
            .header("OData-MaxVersion", "4.0")
            .header("OData-Version", "4.0")
            .header("If-None-Match", "null")
            .header("Accept", "application/json")
            .header("Content-Type", format!("multipart/mixed; boundary={boundary}"))
            .body(body);

        if request.settings.continue_on_error {
            http_request = http_request.header("Prefer", "odata.continue-on-error");
        }

        let resp = http_request
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let status = resp.status();
        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());
        let response_text = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read batch response: {e}"))?;

        if !status.is_success() && !content_type.as_deref().unwrap_or_default().starts_with("multipart/mixed") {
            return Err(format!(
                "Dataverse API error ({}): {}",
                status,
                response_text
            ));
        }

        let parts = parse_batch_response_parts(content_type.as_deref(), &response_text)?;
        self.map_batch_response(request, parts)
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

    async fn entity_set_name_map(&self) -> Result<HashMap<String, String>, String> {
        let definitions = self.list_entity_definitions().await?;
        Ok(definitions
            .into_iter()
            .map(|definition| {
                (
                    definition.logical_name.to_ascii_lowercase(),
                    definition.entity_set_name,
                )
            })
            .collect())
    }

    fn prepare_batch_requests(
        &self,
        requests: &[OrganizationRequest],
        entity_set_name_by_logical_name: &HashMap<String, String>,
    ) -> Result<Vec<PreparedBatchItem>, String> {
        requests
            .iter()
            .enumerate()
            .map(|(request_index, request)| {
                self.prepare_batch_request(
                    request_index,
                    request.clone(),
                    entity_set_name_by_logical_name,
                )
            })
            .collect()
    }

    fn prepare_batch_request(
        &self,
        _request_index: usize,
        request: OrganizationRequest,
        entity_set_name_by_logical_name: &HashMap<String, String>,
    ) -> Result<PreparedBatchItem, String> {
        let prepared = match &request {
            OrganizationRequest::Create(request) => {
                let entity_set_name = entity_set_name_by_logical_name
                    .get(&request.target.logical_name.to_ascii_lowercase())
                    .ok_or_else(|| {
                        format!(
                            "Entity set metadata not found for '{}'",
                            request.target.logical_name
                        )
                    })?;

                PreparedBatchRequest {
                    method: "POST",
                    path: format!("/api/data/v9.2/{entity_set_name}"),
                    body: Some(entity_to_write_body(
                        &request.target,
                        entity_set_name_by_logical_name,
                    )?),
                    parameters: request.parameters.clone(),
                }
            }
            OrganizationRequest::Update(request) => {
                if request.target.id.is_nil() {
                    return Err("UpdateRequest target must include a non-empty entity ID".to_string());
                }

                let entity_set_name = entity_set_name_by_logical_name
                    .get(&request.target.logical_name.to_ascii_lowercase())
                    .ok_or_else(|| {
                        format!(
                            "Entity set metadata not found for '{}'",
                            request.target.logical_name
                        )
                    })?;

                PreparedBatchRequest {
                    method: "PATCH",
                    path: format!(
                        "/api/data/v9.2/{}({})",
                        entity_set_name,
                        request.target.id.as_hyphenated()
                    ),
                    body: Some(entity_to_write_body(
                        &request.target,
                        entity_set_name_by_logical_name,
                    )?),
                    parameters: request.parameters.clone(),
                }
            }
            OrganizationRequest::Delete(request) => {
                let entity_set_name = entity_set_name_by_logical_name
                    .get(&request.target.logical_name.to_ascii_lowercase())
                    .ok_or_else(|| {
                        format!(
                            "Entity set metadata not found for '{}'",
                            request.target.logical_name
                        )
                    })?;

                PreparedBatchRequest {
                    method: "DELETE",
                    path: format!(
                        "/api/data/v9.2/{}({})",
                        entity_set_name,
                        request.target.id.as_hyphenated()
                    ),
                    body: None,
                    parameters: request.parameters.clone(),
                }
            }
        };

        Ok(PreparedBatchItem {
            prepared_request: prepared,
        })
    }

    fn build_batch_body(&self, boundary: &str, requests: &[PreparedBatchItem]) -> String {
        let mut body = String::new();

        for (content_id, item) in requests.iter().enumerate() {
            body.push_str(&format!("--{boundary}\r\n"));
            body.push_str("Content-Type: application/http\r\n");
            body.push_str("Content-Transfer-Encoding: binary\r\n");
            body.push_str(&format!("Content-ID: {}\r\n\r\n", content_id + 1));
            body.push_str(&format!(
                "{} {} HTTP/1.1\r\n",
                item.prepared_request.method, item.prepared_request.path
            ));
            body.push_str("Accept: application/json\r\n");

            for (header, value) in item.prepared_request.parameters.headers() {
                body.push_str(&format!("{header}: {value}\r\n"));
            }

            if let Some(payload) = &item.prepared_request.body {
                body.push_str("Content-Type: application/json;type=entry\r\n\r\n");
                body.push_str(payload);
                body.push_str("\r\n");
            } else {
                body.push_str("\r\n");
            }
        }

        body.push_str(&format!("--{boundary}--\r\n"));
        body
    }

    fn map_batch_response(
        &self,
        request: &ExecuteMultipleRequest,
        parts: Vec<ParsedBatchPart>,
    ) -> Result<ExecuteMultipleResponse, String> {
        let mut response = ExecuteMultipleResponse::default();

        for (part_index, part) in parts.iter().enumerate() {
            let Some(source_request) = request.requests.get(part_index) else {
                break;
            };

            if part.status_code >= 400 {
                response.responses.push(ExecuteMultipleResponseItem {
                    request_index: part_index,
                    response: None,
                    fault: Some(parse_fault(part)),
                });
                continue;
            }

            if request.settings.return_responses {
                response.responses.push(ExecuteMultipleResponseItem {
                    request_index: part_index,
                    response: Some(source_request.success_response(&part.headers)),
                    fault: None,
                });
            }
        }

        Ok(response)
    }
}

fn ensure_fetch_page_size(fetchxml: &str, page_size: i32) -> Result<String, String> {
    if fetch_tag_has_attr(fetchxml, "count")? {
        return Ok(fetchxml.to_string());
    }

    let fetch_start = fetchxml
        .find("<fetch")
        .ok_or_else(|| "FetchXML must start with a <fetch> element".to_string())?;
    let tag_end = fetchxml[fetch_start..]
        .find('>')
        .ok_or_else(|| "FetchXML <fetch> element is not closed".to_string())?
        + fetch_start;

    let mut inserted = String::new();
    inserted.push_str(&fetchxml[..tag_end]);
    inserted.push_str(&format!(" count=\"{page_size}\""));
    inserted.push_str(&fetchxml[tag_end..]);
    Ok(inserted)
}

fn normalize_entity_name(value: &str) -> String {
    value
        .trim_matches(|ch| ch == '[' || ch == ']' || ch == '"' || ch == '`')
        .to_ascii_lowercase()
}

fn parse_uuid_from_uri(value: &str) -> Option<Uuid> {
    let start = value.rfind('(')? + 1;
    let end = value.rfind(')')?;
    Uuid::parse_str(value[start..end].trim_matches('{').trim_matches('}')).ok()
}
