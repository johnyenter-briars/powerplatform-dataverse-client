# powerplatform-dataverse-client

Unofficial Rust sdk for the Microsoft Dataverse (Power Platform) Web API. 

Currently in use as the backend for [Queryverse](https://github.com/johnyenter-briars/queryverse) - the Dataverse SQL client.

The **short term** goal is to provide Rust programs a frictionless, powerful, simple, and fast method of communication with Dataverse.

The **long term** goal for this project is feature parity with the [Microsoft.PowerPlatform.Dataverse.Client](https://www.nuget.org/packages/Microsoft.PowerPlatform.Dataverse.Client).

## Features

| Feature | Supported |
| --- | --- |
| [Client-credentials auth](doc/client-credentials-auth.md) | ✅ |
| Authorization code / password grant token exchange | ❌ |
| [Device code auth](doc/device-code-auth.md) | ✅ |
| [Automatic token refresh](doc/token-refresh.md) | ✅ |
| [Token cache](doc/token-cache.md) | ✅ |
| [FetchXML retrieval](doc/fetchxml.md) | ✅ |
| [FetchXML paging](doc/fetchxml.md) | ✅ |
| [FetchXML count helper](doc/fetchxml.md) | ✅ |
| [Entity definitions metadata](doc/metadata.md) | ✅ |
| [Entity attributes metadata](doc/metadata.md) | ✅ |
| [Entity relationships metadata](doc/metadata.md) | ✅ |
| Entity identity fields (id/logical/name via convention) | ✅ |
| Create entity | ✅ |
| Update entity by ID | ✅ |
| Delete entity by ID | ✅ |
| Batch operations (`ExecuteMultiple`-style) | ✅ |
| Request / Response Objects (`CreateRequest`, `UpdateRequest`, `DeleteRequest`, `ExecuteMultipleRequest`) | ✅ |
| Bypass Custom Logic params | ✅ |
| Retrieve entity by ID | ❌ |
| Entity multi-identity fields | ❌ |
| Retry/backoff | ❌ |
| OData query syntax (non-FetchXML) | ❌ |
| Expanded navigation properties | ❌ |

## Quick Start

```rust
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;
use powerplatform_dataverse_client::LogLevel;

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

Feature documentation:

- [Client credentials auth](doc/client-credentials-auth.md)
- [Device code auth](doc/device-code-auth.md)
- [FetchXML](doc/fetchxml.md)
- [Metadata](doc/metadata.md)
- [Token refresh](doc/token-refresh.md)
- [Token cache](doc/token-cache.md)

## Supported Methods

### Dataverse Service Client
- `new`
- `new_with_auth`
- `retrieve_multiple_fetchxml`
- `retrieve_multiple_fetchxml_paging`
- `retrieve_multiple_fetchxml_count`
- `list_entity_definitions`
- `list_entity_attributes`
- `list_entity_relationships`
- `update_entity`
- `update_entity_with_options`
- `create_entity`
- `create_entity_with_options`
- `delete_entity`
- `delete_entity_with_options`
- `execute_multiple`

## ExecuteMultiple

The client now includes SDK-shaped request/response types for batched write operations:

- `CreateRequest`
- `UpdateRequest`
- `DeleteRequest`
- `ExecuteMultipleRequest`
- `ExecuteMultipleResponse`

These requests are executed through the Dataverse Web API `$batch` endpoint and mirror the broad behavior of the .NET SDK `ExecuteMultiple` pattern:

- requests run in order
- `continue_on_error` maps to the batch `Prefer: odata.continue-on-error` header
- `return_responses` controls whether successful items are surfaced in the client response

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

## RequestParameters

Use `RequestParameters` with `update_entity_with_options` and `delete_entity_with_options` to send optional Dataverse request headers on create, update, and delete operations.

```rust
use powerplatform_dataverse_client::dataverse::requestparameters::RequestParameters;

let request_parameters = RequestParameters {
    bypass_business_logic_execution_custom_sync: true,
    bypass_business_logic_execution_custom_async: false,
    bypass_custom_plugin_execution: false,
    suppress_callback_registration_expander_job: true,
};
```

| Request Parameter | Header | Values | Status |
| --- | --- | --- | --- |
| `BypassBusinessLogicExecution` | `MSCRM.BypassBusinessLogicExecution` | `CustomSync`, `CustomAsync`, `CustomSync,CustomAsync` | ✅ |
| `BypassBusinessLogicExecutionStepIds` | `MSCRM.BypassBusinessLogicExecutionStepIds` | Comma-separated plug-in step registration IDs | ⏳ TODO |
| `BypassCustomPluginExecution` | `MSCRM.BypassCustomPluginExecution` | `true` | ✅ |
| `SuppressCallbackRegistrationExpanderJob` | `MSCRM.SuppressCallbackRegistrationExpanderJob` | `true` | ✅ |


## Data Types

| Data Type | Supported | Rust Type |
| --- | --- | --- |
| GUID / Primary Entity ID | ✅ | `Uuid` |
| `Int` | ✅ | `Value::Int(i64)` |
| `Float` | ✅ | `Value::Float(f64)` |
| `Decimal` | ✅ | `Value::Decimal(Decimal)` |
| `String` | ✅ | `Value::String(String)` |
| `Boolean` | ✅ | `Value::Boolean(bool)` |
| Date/time | ✅ | `Value::DateTime(DateTime<Utc>)` |
| GUID column | ✅ | `Value::Guid(Uuid)` |
| Money | ✅ | `Value::Money(Money)` |
| Option sets / labeled values | ✅ | `Value::OptionSetValue(OptionSetValue)` |
| Multi-select | ✅ | `Value::OptionSetValueCollection(OptionSetValueCollection)` |
| `Null` | ✅ | `Value::Null` |
| Lookups / entity references | ✅ | `Value::EntityReference(EntityReference)` |
| Polymorphic lookups | ✅ | `Value::EntityReference(EntityReference)` |
| Aliased values | ❌ | N/A |
| EntityList | ❌ | N/A |

## Samples

```powershell
cd samples/<sample-name>
cp secrets.example.json secrets.json #populate secrets.json with auth information
cargo run
```

[hello-dataverse](samples/hello-dataverse/README.md) is the smallest sample.

[v1-features](samples/v1-features/README.md) contains one scenario per feature, all launched from `main`:

- [Device code auth scenario](samples/v1-features/src/scenarios/device_code_auth.rs)
- [Client credentials auth scenario](samples/v1-features/src/scenarios/client_credentials_auth.rs)
- [Metadata scenario](samples/v1-features/src/scenarios/metadata.rs)
- [Data types scenario](samples/v1-features/src/scenarios/data_types.rs)
- [FetchXML scenario](samples/v1-features/src/scenarios/fetchxml.rs)
- [Refresh demo scenario](samples/v1-features/src/scenarios/refresh_demo.rs)
- [Batch scenario](samples/v1-features/src/scenarios/batch.rs)

## Contributing

Issues and pull requests are welcome. Please include a brief description of the change and, when possible, add or update tests.

## AI Disclosure
Portions of this project were developed with the assistance of AI tools; all changes are reviewed and tested by maintainers.

## License

See [LICENSE](LICENSE).
