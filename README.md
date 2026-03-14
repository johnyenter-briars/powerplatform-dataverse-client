# powerplatform-dataverse-client

Unofficial Rust sdk for the Microsoft Dataverse (Power Platform) Web API. 

Currently in use as the backend for [Queryverse](https://github.com/johnyenter-briars/queryverse) - the Dataverse SQL client.

The **long term** goal for this project is feature parity with the [Microsoft.PowerPlatform.Dataverse.Client](https://www.nuget.org/packages/Microsoft.PowerPlatform.Dataverse.Client).

The **short term** goal is to provide Rust programs a frictionless, powerful, simple, and fast method of communication with Dataverse.

## Features

| Feature | Supported |
| --- | --- |
| Client-credentials auth | ✅ |
| Authorization code / password grant token exchange | ❌ |
| Device code auth | ✅ |
| Automatic token refresh (auth code flow) | ❌ |
| FetchXML retrieval | ✅ |
| FetchXML paging | ✅ |
| FetchXML count helper | ✅ |
| Entity definitions metadata | ✅ |
| Entity attributes metadata | ✅ |
| Entity identity fields (id/logical/name via convention) | ✅ |
| Update entity by ID | ✅ |
| Delete entity by ID | ✅ |
| Bypass Custom Logic params | ✅ |
| Create entity | ❌ |
| Retrieve entity by ID | ❌ |
| Entity multi-identity fields | ❌ |
| Batch operations | ❌ |
| Retry/backoff | ❌ |
| Request / Response Objects (Replicating the C# SDK) | ❌ |
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
        .retrieve_multiple_fetchxml("accounts", fetchxml)
        .await?;

    println!("Records: {}", entities.len());
    Ok(())
}
```

## Supported Methods

### Authentication
- `fetch_client_credentials_token`
- `fetch_client_credentials_token_with_expiry`
- `fetch_device_code_token`
- `validate_client_credentials`
- `exchange_authorization_code` (authorization code or password grant)

### Dataverse Service Client
- `retrieve_multiple_fetchxml`
- `retrieve_multiple_fetchxml_count`
- `list_entity_definitions`
- `list_entity_attributes`
- `update_entity`
- `update_entity_with_options`
- `delete_entity`
- `delete_entity_with_options`

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
| `String` | ✅ | `Value::String(String)` |
| `Boolean` | ✅ | `Value::Boolean(bool)` |
| `Null` | ✅ | `Value::Null` |
| Lookups / entity references | ✅ | `Value::EntityReference(EntityReference)` |
| Polymorphic lookups | ⏳ | N/A |
| Option sets / labeled values | ⏳ | N/A |
| Multi-select | ⏳ | N/A |
| Date/time | ⏳ | N/A |
| Floating Point Number | ⏳ | N/A |
| Money | ⏳ | N/A |
| Aliased values | ❌ | N/A |
| EntityList | ❌ | N/A |

## Samples

```powershell
cd samples/<sample-name>
cp secrets.example.json secrets.json #populate secrets.json with auth information
cargo run
```

`hello-dataverse` also supports a Dataverse-style device-code connection string in `secrets.json`:

```json
{
  "device_code_connection_string": "AuthType=OAuth;Url=https://contosotest.crm.dynamics.com;AppId=51f81489-12ee-4a9e-aaae-a2591f45987d;RedirectUri=app://58145B91-0C36-4500-8554-080854F2AC97;LoginPrompt=Auto;TokenCacheStorePath=C:\\MyTokenCache",
  "client_credentials_connection_string": "AuthType=ClientSecret;Url=https://contosotest.crm.dynamics.com;ClientId=00000000-0000-0000-0000-000000000000;ClientSecret=YOUR_SECRET;TenantId=YOUR_TENANT_ID;TokenCacheStorePath=C:\\MyTokenCache\\token-cache.txt"
}
```

When `device_code_connection_string` is set, the sample prints the Microsoft sign-in URL and waits for the browser sign-in to complete before continuing.

If `TokenCacheStorePath` is set to a folder, the client stores `token_cache.txt` in that folder. If it is set to a file path, the client writes the cache JSON to that file directly. If omitted, the client uses `data_local_dir()/powerplatform-dataverse-client/<derived-guid>/token_cache.txt`.

## Contributing

Issues and pull requests are welcome. Please include a brief description of the change and, when possible, add or update tests.

## License

See [LICENSE](LICENSE).
