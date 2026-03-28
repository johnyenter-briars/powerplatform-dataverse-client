# Service Client

`ServiceClient` is the crate's main Dataverse Web API entry point.

Microsoft Learn background:

- [Use the Microsoft Dataverse Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/overview)
- [Use FetchXML to retrieve data](https://learn.microsoft.com/power-apps/developer/data-platform/fetchxml/overview)
- [Query table definitions using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/query-metadata-web-api)
- [Create and update table rows using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/create-entity-web-api)
- [Delete table rows using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/delete-entity-using-web-api)
- [Execute batch operations using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/execute-batch-operations-using-web-api)

## Public API

### Constructors

- `ServiceClient::new(connection_string: &str, log_level: LogLevel) -> Result<ServiceClient, String>`
- `ServiceClient::new_with_auth(auth: AuthConfig, log_level: LogLevel) -> Result<ServiceClient, String>`

### Auth state

- `ServiceClient::token_expires_at(&self) -> Option<DateTime<Utc>>`

### FetchXML retrieval

- `ServiceClient::retrieve_multiple_fetchxml(&self, entity: &str, fetchxml: &str) -> Result<Vec<Entity>, String>`
- `ServiceClient::retrieve_multiple_fetchxml_paging(&self, entity: &str, fetchxml: &str) -> Result<Vec<Entity>, String>`
- `ServiceClient::retrieve_multiple_fetchxml_paging_with_progress(&self, entity: &str, fetchxml: &str, on_progress: F, page_size: Option<i32>) -> Result<Vec<Entity>, String>`
- `ServiceClient::retrieve_multiple_fetchxml_count(&self, entity: &str, fetchxml: &str) -> Result<usize, String>`

### Metadata

- `ServiceClient::list_entity_definitions(&self) -> Result<Vec<EntityDefinition>, String>`
- `ServiceClient::list_entity_attributes(&self, logical_name: &str) -> Result<Vec<EntityAttribute>, String>`
- `ServiceClient::list_entity_relationships(&self, logical_name: &str) -> Result<Vec<EntityRelationship>, String>`

### CRUD

- `ServiceClient::create_entity(&self, entity_set: &str, attributes: &HashMap<String, serde_json::Value>) -> Result<Option<Uuid>, String>`
- `ServiceClient::create_entity_with_options(&self, entity_set: &str, attributes: &HashMap<String, serde_json::Value>, options: &RequestParameters) -> Result<Option<Uuid>, String>`
- `ServiceClient::update_entity(&self, entity_set: &str, id: &str, attributes: &HashMap<String, serde_json::Value>) -> Result<(), String>`
- `ServiceClient::update_entity_with_options(&self, entity_set: &str, id: &str, attributes: &HashMap<String, serde_json::Value>, options: &RequestParameters) -> Result<(), String>`
- `ServiceClient::delete_entity(&self, entity_set: &str, id: &str) -> Result<(), String>`
- `ServiceClient::delete_entity_with_options(&self, entity_set: &str, id: &str, options: &RequestParameters) -> Result<(), String>`

### Batch

- `ServiceClient::execute_multiple(&self, request: &ExecuteMultipleRequest) -> Result<ExecuteMultipleResponse, String>`

## Notes

- `ServiceClient` handles token acquisition, token refresh, and cache persistence internally.
- FetchXML helpers prefer Dataverse-shaped behavior rather than trying to be a generic OData client.
- Metadata calls are cached inside the client because intellisense, schema browsing, and write shaping tend to reuse the same entity metadata heavily.
- CRUD methods accept `serde_json::Value` maps so callers can assemble lightweight payloads without first materializing `Entity`.

## Related Pages

- [FetchXML](fetchxml.md)
- [Metadata](metadata.md)
- [Request parameters](request-parameters.md)
- [Batch](batch.md)
