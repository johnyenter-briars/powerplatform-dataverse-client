# powerplatform-dataverse-client

Unofficial Rust SDK for the Microsoft Dataverse (Power Platform) Web API.

This crate is used as the Dataverse backend for [Queryverse](https://github.com/johnyenter-briars/queryverse). The immediate goal is a practical, low-friction Rust client for real Dataverse work. The longer-term goal is to cover the most useful parts of the `Microsoft.PowerPlatform.Dataverse.Client` experience with a Rust-shaped API.

## Microsoft Learn

The crate centers on the Dataverse Web API and these Microsoft Learn references:

- [Use the Microsoft Dataverse Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/overview)
- [Authenticate with Dataverse Web API](https://learn.microsoft.com/power-apps/developer/data-platform/authenticate-oauth)
- [Use FetchXML to retrieve data](https://learn.microsoft.com/power-apps/developer/data-platform/fetchxml/overview)
- [Query table definitions using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/query-metadata-web-api)
- [Create and update table rows using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/create-entity-web-api)
- [Delete table rows using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/delete-entity-using-web-api)
- [Execute batch operations using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/execute-batch-operations-using-web-api)
- [Bypass custom business logic](https://learn.microsoft.com/power-apps/developer/data-platform/bypass-custom-business-logic)

## Features

| Feature | Supported |
| --- | --- |
| Client-credentials auth | ✅ |
| Device code auth | ✅ |
| Automatic token refresh | ✅ |
| Token cache | ✅ |
| FetchXML retrieval | ✅ |
| FetchXML paging | ✅ |
| FetchXML paging progress callback | ✅ |
| FetchXML count helper | ✅ |
| Entity definitions metadata | ✅ |
| Entity attributes metadata | ✅ |
| Entity relationships metadata | ✅ |
| Create entity | ✅ |
| Update entity by ID | ✅ |
| Delete entity by ID | ✅ |
| Batch operations (`ExecuteMultiple`-style) | ✅ |
| Dataverse request-parameter headers | ✅ |
| Retrieve entity by ID | ❌ |
| OData query syntax (non-FetchXML) | ❌ |
| Retry/backoff | ❌ |

## Quick Start

```rust
use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

#[tokio::main]
async fn main() -> Result<(), String> {
    let client = ServiceClient::new(
        "AuthType=ClientSecret;Url=https://YOUR_ORG.crm.dynamics.com;ClientId=CLIENT_ID;ClientSecret=CLIENT_SECRET;TenantId=TENANT_ID;TokenCacheStorePath=C:\\MyTokenCache\\token-cache.txt",
        LogLevel::Information,
    )
    .await?;

    let fetchxml = r#"
        <fetch top="5">
          <entity name="account">
            <attribute name="accountid" />
            <attribute name="name" />
          </entity>
        </fetch>
    "#;

    let entities = client
        .retrieve_multiple_fetchxml_paging("accounts", fetchxml)
        .await?;

    println!("Records: {}", entities.len());
    Ok(())
}
```

Additional feature notes live in:

- [Client credentials auth](doc/client-credentials-auth.md)
- [Device code auth](doc/device-code-auth.md)
- [FetchXML](doc/fetchxml.md)
- [Metadata](doc/metadata.md)
- [Token refresh](doc/token-refresh.md)
- [Token cache](doc/token-cache.md)

## Public API

This section lists every public struct, enum, type alias, and method exported by the crate.

### Logging

Related Learn area:
- Dataverse requests themselves do not define logging behavior; `LogLevel` is crate-local request logging.

Public types:
- `LogLevel`: SDK log verbosity enum.

Public methods:
- `LogLevel::as_filter(self) -> log::LevelFilter`
- `LogLevel::includes_debug(self) -> bool`

### Authentication

Related Learn area:
- [Authenticate with Dataverse Web API](https://learn.microsoft.com/power-apps/developer/data-platform/authenticate-oauth)

Public types:
- `AuthConfig`: auth configuration enum with `ClientCredentials` and `DeviceCode` variants.
- `DeviceCodeFlowEvent`: progress events emitted during device-code sign-in.
- `TokenExchange`: access token, refresh token, and expiry returned from refresh/device-code exchange paths.
- `ClientCredentialsToken`: access token and expiry returned from client-credentials auth.

Public methods:
- `AuthConfig::from_connection_string(connection_string: &str) -> Result<AuthConfig, String>`
- `ensure_device_code_token_with_progress(auth: &AuthConfig, progress: F) -> Result<(), String>`

### Dataverse Service Client

Related Learn areas:
- [Use the Microsoft Dataverse Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/overview)
- [Use FetchXML to retrieve data](https://learn.microsoft.com/power-apps/developer/data-platform/fetchxml/overview)
- [Query table definitions using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/query-metadata-web-api)
- [Create and update table rows using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/create-entity-web-api)
- [Delete table rows using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/delete-entity-using-web-api)
- [Execute batch operations using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/execute-batch-operations-using-web-api)

Public types:
- `ServiceClient`: main Dataverse client.

Public methods:
- `ServiceClient::new(connection_string: &str, log_level: LogLevel) -> Result<ServiceClient, String>`
- `ServiceClient::new_with_auth(auth: AuthConfig, log_level: LogLevel) -> Result<ServiceClient, String>`
- `ServiceClient::token_expires_at(&self) -> Option<DateTime<Utc>>`
- `ServiceClient::retrieve_multiple_fetchxml(&self, entity: &str, fetchxml: &str) -> Result<Vec<Entity>, String>`
- `ServiceClient::retrieve_multiple_fetchxml_paging(&self, entity: &str, fetchxml: &str) -> Result<Vec<Entity>, String>`
- `ServiceClient::retrieve_multiple_fetchxml_paging_with_progress(&self, entity: &str, fetchxml: &str, on_progress: F, page_size: Option<i32>) -> Result<Vec<Entity>, String>`
- `ServiceClient::retrieve_multiple_fetchxml_count(&self, entity: &str, fetchxml: &str) -> Result<usize, String>`
- `ServiceClient::list_entity_definitions(&self) -> Result<Vec<EntityDefinition>, String>`
- `ServiceClient::list_entity_attributes(&self, logical_name: &str) -> Result<Vec<EntityAttribute>, String>`
- `ServiceClient::list_entity_relationships(&self, logical_name: &str) -> Result<Vec<EntityRelationship>, String>`
- `ServiceClient::update_entity(&self, entity_set: &str, id: &str, attributes: &HashMap<String, serde_json::Value>) -> Result<(), String>`
- `ServiceClient::create_entity(&self, entity_set: &str, attributes: &HashMap<String, serde_json::Value>) -> Result<Option<Uuid>, String>`
- `ServiceClient::create_entity_with_options(&self, entity_set: &str, attributes: &HashMap<String, serde_json::Value>, options: &RequestParameters) -> Result<Option<Uuid>, String>`
- `ServiceClient::update_entity_with_options(&self, entity_set: &str, id: &str, attributes: &HashMap<String, serde_json::Value>, options: &RequestParameters) -> Result<(), String>`
- `ServiceClient::delete_entity(&self, entity_set: &str, id: &str) -> Result<(), String>`
- `ServiceClient::delete_entity_with_options(&self, entity_set: &str, id: &str, options: &RequestParameters) -> Result<(), String>`
- `ServiceClient::execute_multiple(&self, request: &ExecuteMultipleRequest) -> Result<ExecuteMultipleResponse, String>`

### Request Parameters

Related Learn area:
- [Bypass custom business logic](https://learn.microsoft.com/power-apps/developer/data-platform/bypass-custom-business-logic)

Public types:
- `RequestParameters`: optional Dataverse request-header flags for create, update, and delete operations.

Public methods:
- `RequestParameters::headers(&self) -> Vec<(&'static str, &'static str)>`
- `RequestParameters::apply(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder`

Currently supported request headers:

| Field | Dataverse Header | Supported |
| --- | --- | --- |
| `bypass_business_logic_execution_custom_sync` | `MSCRM.BypassBusinessLogicExecution=CustomSync` | ✅ |
| `bypass_business_logic_execution_custom_async` | `MSCRM.BypassBusinessLogicExecution=CustomAsync` | ✅ |
| `bypass_custom_plugin_execution` | `MSCRM.BypassCustomPluginExecution=true` | ✅ |
| `suppress_callback_registration_expander_job` | `MSCRM.SuppressCallbackRegistrationExpanderJob=true` | ✅ |

### Batch Request and Response Types

Related Learn area:
- [Execute batch operations using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/execute-batch-operations-using-web-api)

Public types:
- `ExecuteMultipleSettings`
- `ExecuteMultipleRequest`
- `ExecuteMultipleResponse`
- `ExecuteMultipleResponseItem`
- `OrganizationServiceFault`
- `OrganizationRequest`
- `OrganizationResponse`
- `CreateRequest`
- `CreateResponse`
- `UpdateRequest`
- `UpdateResponse`
- `DeleteRequest`
- `DeleteResponse`

Public methods:
- `CreateRequest::new(target: Entity) -> CreateRequest`
- `UpdateRequest::new(target: Entity) -> UpdateRequest`
- `DeleteRequest::new(target: EntityReference) -> DeleteRequest`

Example:

```rust
use powerplatform_dataverse_client::dataverse::batch::{
    ExecuteMultipleRequest, ExecuteMultipleSettings, OrganizationRequest, UpdateRequest,
};
use powerplatform_dataverse_client::dataverse::entity::{Entity, Value};

let mut account = Entity::new(account_id, "account", None);
account
    .attributes
    .insert("tickersymbol".to_string(), Value::String("MSFT".to_string()));

let response = client
    .execute_multiple(&ExecuteMultipleRequest {
        settings: ExecuteMultipleSettings {
            continue_on_error: false,
            return_responses: false,
        },
        requests: vec![OrganizationRequest::Update(UpdateRequest::new(account))],
    })
    .await?;
```

### Entity and Value Types

Related Learn area:
- [Use the Microsoft Dataverse Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/overview)

Public types:
- `Value`
- `Money`
- `OptionSetValue`
- `OptionSetValueCollection`
- `EntityReference`
- `Attribute`
- `Entity`

Public methods:
- `Entity::new(id: Uuid, logical_name: impl Into<String>, name: Option<String>) -> Entity`

Supported value shapes:

| Dataverse Shape | Rust Shape |
| --- | --- |
| Whole numbers | `Value::Int(i64)` |
| Floating-point numbers | `Value::Float(f64)` |
| Exact decimals | `Value::Decimal(Decimal)` |
| Strings | `Value::String(String)` |
| Booleans | `Value::Boolean(bool)` |
| Date/time values | `Value::DateTime(DateTime<Utc>)` |
| GUID attributes | `Value::Guid(Uuid)` |
| Money values | `Value::Money(Money)` |
| Single-choice values | `Value::OptionSetValue(OptionSetValue)` |
| Multi-select values | `Value::OptionSetValueCollection(OptionSetValueCollection)` |
| Nulls | `Value::Null` |
| Lookups | `Value::EntityReference(EntityReference)` |

### Metadata Types

Related Learn area:
- [Query table definitions using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/query-metadata-web-api)

Public types:
- `AttributeTypeName`
- `EntityAttribute`
- `EntityDefinition`
- `EntityRelationship`

## Samples

```powershell
cd samples/<sample-name>
cp secrets.example.json secrets.json
cargo run
```

[hello-dataverse](samples/hello-dataverse/README.md) is the smallest sample.

[v1-features](samples/v1-features/README.md) contains one scenario per feature:

- [Device code auth scenario](samples/v1-features/src/scenarios/device_code_auth.rs)
- [Device code progress scenario](samples/v1-features/src/scenarios/device_code_progress.rs)
- [Client credentials auth scenario](samples/v1-features/src/scenarios/client_credentials_auth.rs)
- [Metadata scenario](samples/v1-features/src/scenarios/metadata.rs)
- [Data types scenario](samples/v1-features/src/scenarios/data_types.rs)
- [FetchXML scenario](samples/v1-features/src/scenarios/fetchxml.rs)
- [CRUD scenario](samples/v1-features/src/scenarios/crud.rs)
- [Request parameters scenario](samples/v1-features/src/scenarios/request_parameters.rs)
- [Batch scenario](samples/v1-features/src/scenarios/batch.rs)
- [Refresh demo scenario](samples/v1-features/src/scenarios/refresh_demo.rs)

## Contributing

Issues and pull requests are welcome. Please include a brief description of the change and, when possible, add or update tests.

## AI Disclosure

Portions of this project were developed with the assistance of AI tools; all changes are reviewed and tested by maintainers.

## License

See [LICENSE](LICENSE).
