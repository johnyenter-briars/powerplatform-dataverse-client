use std::collections::HashMap;

use reqwest::Client;
use serde_json::Value;

use crate::dataverse::entity::Value::Int;
use crate::dataverse::entity::Entity;
use crate::dataverse::entityattribute::EntityAttribute;
use crate::dataverse::entitydefinition::EntityDefinition;
use crate::dataverse::fetchxml::{apply_paging, ensure_aggregate_page_size, fetch_tag_has_attr};
use crate::dataverse::parse::{
    extract_paging_cookie, parse_entities_from_response, parse_more_records,
    parse_record_count_from_response,
};
use crate::LogLevel;

const ROW_NUMBER_ATTRIBUTE: &str = "__rownum";
const AGGREGATE_PAGE_SIZE: i32 = 5000;

/// OData list wrapper returned by Dataverse metadata endpoints.
#[derive(Debug, serde::Deserialize)]
struct ODataList<T> {
    value: Vec<T>,
}

/// HTTP client for Dataverse Web API operations.
pub struct ServiceClient {
    client: Client,
    base_url: std::string::String,
    token: std::string::String,
    log_level: LogLevel,
}

impl ServiceClient {
    /// Create a new client for the given base URL and access token.
    pub fn new(base_url: &str, token: &str, log_level: LogLevel) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
            log_level,
        }
    }

    /// Retrieve multiple records by FetchXML, handling paging when needed.
    pub async fn retrieve_multiple_fetchxml(
        &self,
        entity: &str,
        fetchxml: &str,
    ) -> Result<Vec<Entity>, std::string::String> {
        if fetch_tag_has_attr(fetchxml, "top")? {
            return self
                .retrieve_multiple_fetchxml_single(entity, fetchxml)
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

            if matches!(self.log_level, LogLevel::Debug) {
                println!("Fetch page: {}", page);
                println!("FetchXML: {}", fetch_with_paging);
            }

            let mut url = format!("{}/api/data/v9.2/{}", self.base_url, entity);
            url.push_str("?fetchXml=");
            url.push_str(&urlencoding::encode(&fetch_with_paging));

            if matches!(self.log_level, LogLevel::Debug) {
                println!("Url: {:?}", url);
            }

            let resp = self
                .client
                .get(&url)
                .bearer_auth(&self.token)
                .header("Accept", "application/json")
                .header(
                    "Prefer",
                    "odata.include-annotations=\"Microsoft.Dynamics.CRM.fetchxmlpagingcookie,Microsoft.Dynamics.CRM.morerecords\"",
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

            let mut page_entities = parse_entities_from_response(&json)?;
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
                .retrieve_multiple_fetchxml_single(entity, fetchxml)
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

            if matches!(self.log_level, LogLevel::Debug) {
                println!("Fetch page: {}", page);
                println!("FetchXML: {}", fetch_with_paging);
            }

            let mut url = format!("{}/api/data/v9.2/{}", self.base_url, entity);
            url.push_str("?fetchXml=");
            url.push_str(&urlencoding::encode(&fetch_with_paging));

            if matches!(self.log_level, LogLevel::Debug) {
                println!("Url: {:?}", url);
            }

            let resp = self
                .client
                .get(&url)
                .bearer_auth(&self.token)
                .header("Accept", "application/json")
                .header(
                    "Prefer",
                    "odata.include-annotations=\"Microsoft.Dynamics.CRM.fetchxmlpagingcookie,Microsoft.Dynamics.CRM.morerecords\"",
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
    ) -> Result<Vec<Entity>, std::string::String> {
        if matches!(self.log_level, LogLevel::Debug) {
            println!("FetchXML: {}", fetchxml);
        }

        let mut url = format!("{}/api/data/v9.2/{}", self.base_url, entity);
        url.push_str("?fetchXml=");
        url.push_str(&urlencoding::encode(fetchxml));

        if matches!(self.log_level, LogLevel::Debug) {
            println!("Url: {:?}", url);
        }

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .header("Accept", "application/json")
            .header(
                "Prefer",
                "odata.include-annotations=\"Microsoft.Dynamics.CRM.fetchxmlpagingcookie,Microsoft.Dynamics.CRM.morerecords\"",
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

        parse_entities_from_response(&json)
    }

    /// List all entity definitions.
    pub async fn list_entity_definitions(
        &self,
    ) -> Result<Vec<EntityDefinition>, std::string::String> {
        let url = format!(
            "{}/api/data/v9.2/EntityDefinitions?$select=LogicalName,SchemaName,DisplayName,EntitySetName,IsCustomEntity,PrimaryIdAttribute",
            self.base_url
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
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

        Ok(parsed.value)
    }

    /// List entity attributes for a given logical name.
    pub async fn list_entity_attributes(
        &self,
        logical_name: &str,
    ) -> Result<Vec<EntityAttribute>, std::string::String> {
        let logical = logical_name.replace('\'', "''");
        let url = format!(
            "{}/api/data/v9.2/EntityDefinitions(LogicalName='{}')/Attributes?$select=LogicalName,SchemaName,AttributeType,IsCustomAttribute,IsValidODataAttribute,IsValidForRead,IsValidForUpdate&$filter=IsValidODataAttribute eq true and IsValidForRead eq true",
            self.base_url, logical
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
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

        Ok(parsed.value)
    }

    /// Update a single entity record by ID.
    pub async fn update_entity(
        &self,
        entity_set: &str,
        id: &str,
        attributes: &HashMap<std::string::String, Value>,
    ) -> Result<(), std::string::String> {
        let trimmed = id.trim_matches(|ch| ch == '{' || ch == '}');
        let url = format!(
            "{}/api/data/v9.2/{}({})",
            self.base_url, entity_set, trimmed
        );

        let resp = self
            .client
            .patch(&url)
            .bearer_auth(&self.token)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&attributes)
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
        let trimmed = id.trim_matches(|ch| ch == '{' || ch == '}');
        let url = format!(
            "{}/api/data/v9.2/{}({})",
            self.base_url, entity_set, trimmed
        );

        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&self.token)
            .header("Accept", "application/json")
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
}
