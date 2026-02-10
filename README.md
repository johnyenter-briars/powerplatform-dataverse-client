# powerplatform-dataverse-client

Unofficial Rust sdk for the Microsoft Dataverse (Power Platform) Web API. 

Currently in use as the backend for [Queryverse](github.com/johnyenter-briars/queryverse) - the Dataverse SQL client.

The **long term** goal for this project is feature parity with the [Microsoft.PowerPlatform.Dataverse.Client](https://www.nuget.org/packages/Microsoft.PowerPlatform.Dataverse.Client).

The **short term** goal is to provide Rust programs a frictionless, powerful, and fast method of communication with Dataverse.

## Quick Start

```rust
use powerplatform_dataverse_client::auth::credentials::fetch_client_credentials_token;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;
use powerplatform_dataverse_client::LogLevel;

#[tokio::main]
async fn main() -> Result<(), String> {
    let token = fetch_client_credentials_token(
        "CLIENT_ID",
        "CLIENT_SECRET",
        "TENANT_ID",
        "https://YOUR_ORG.crm.dynamics.com/.default",
    )
    .await?;

    let client = ServiceClient::new(
        "https://YOUR_ORG.crm.dynamics.com",
        &token,
        LogLevel::Information,
    );

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
- `validate_client_credentials`
- `exchange_authorization_code` (authorization code or password grant)

### Dataverse Service Client
- `retrieve_multiple_fetchxml`
- `retrieve_multiple_fetchxml_count`
- `list_entity_definitions`
- `list_entity_attributes`
- `update_entity`
- `delete_entity`

## Features

| Feature | Supported |
| --- | --- |
| Client-credentials auth | ✅ |
| Authorization code / password grant token exchange | ✅ |
| Refresh-token flow | ❌ |
| Automatic token refresh (auth code flow) | ❌ |
| FetchXML retrieval | ✅ |
| FetchXML paging | ✅ |
| FetchXML count helper | ✅ |
| Entity definitions metadata | ✅ |
| Entity attributes metadata | ✅ |
| Update entity by ID | ✅ |
| Delete entity by ID | ✅ |
| Create entity | ❌ |
| Retrieve entity by ID | ❌ |
| Batch operations | ❌ |
| Retry/backoff | ❌ |
| Request / Response Objects (Replicating the C# SDK) | ❌ |
| OData query syntax (non-FetchXML) | ❌ |
| Expanded navigation properties | ❌ |

## Data Types

| Data Type | Supported | Rust Type |
| --- | --- | --- |
| `Int` | ✅ | `Value::Int(i64)` |
| `Float` | ✅ | `Value::Float(f64)` |
| `String` | ✅ | `Value::String(String)` |
| `Boolean` | ✅ | `Value::Boolean(bool)` |
| `Null` | ✅ | `Value::Null` |
| GUID / Entity ID | ❌ | N/A |
| Date/time | ❌ | N/A |
| Option sets / labeled values | ❌ | N/A |
| Money | ❌ | N/A |
| Lookups / entity references | ❌ | N/A |
| Aliased values | ❌ | N/A |
| Complex types | ❌ | N/A |
| Collections / arrays | ❌ | N/A |

## Samples

```powershell
cd samples/<sample-name>
cp secrets.example.json secrets.json #populate secrets.json with auth information
cargo run
```

## Integration Tests

```powershell
cd integration-tests
cp secrets.example.json secrets.json #populate secrets.json with auth information
cargo test
```

## Contributing

Issues and pull requests are welcome. Please include a brief description of the change and, when possible, add or update tests.

## License

See [LICENSE](LICENSE).
