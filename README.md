# powerplatform-dataverse-client

Unofficial Rust SDK for the Microsoft [Dataverse (Power Platform)](https://learn.microsoft.com/en-us/power-apps/maker/data-platform/data-platform-intro) Web API.

The short-term goal is to provide Rust developers a simple yet robust SDK to building integrations with Dataverse.

The long-term goal is full feature parity with the [Microsoft.PowerPlatform.Dataverse.Client](https://learn.microsoft.com/en-us/dotnet/api/microsoft.powerplatform.dataverse.client?view=dataverse-sdk-latest). 

`powerplatform-dataverse-client` is currently used as the Dataverse backend for [Queryverse](https://github.com/johnyenter-briars/queryverse).

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
| Username / Password auth | ❌ |
| Retry/backoff | ❌ |
| Full feature parity with the XRM SDK | ❌ |

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

## Documentation Index

### Dataverse Service Client

`ServiceClient` is the main Dataverse Web API entry point for FetchXML, metadata, CRUD, and batch operations.

See [doc/service-client.md](doc/service-client.md).

### Entity and Value Types

The crate exposes typed Dataverse row/value shapes such as `Entity`, `EntityReference`, `Money`, and `Value`.

See [doc/entity-types.md](doc/entity-types.md).

### Metadata Types

The crate exposes `EntityDefinition`, `EntityAttribute`, `AttributeTypeName`, and `EntityRelationship` for schema-driven workflows.

See [doc/metadata-types.md](doc/metadata-types.md) and [doc/metadata.md](doc/metadata.md).


### Logging

`LogLevel` controls the crate's own request/debug verbosity.

See [doc/logging.md](doc/logging.md).

### Authentication

Authentication centers on `AuthConfig`, device-code progress events, token refresh, and token cache handling.

See:

- [doc/authentication.md](doc/authentication.md)
- [doc/client-credentials-auth.md](doc/client-credentials-auth.md)
- [doc/device-code-auth.md](doc/device-code-auth.md)
- [doc/token-refresh.md](doc/token-refresh.md)
- [doc/token-cache.md](doc/token-cache.md)

### Request Parameters

`RequestParameters` maps supported Dataverse optional request headers onto create, update, and delete operations.

See [doc/request-parameters.md](doc/request-parameters.md).

### Batch Operations

Batch operations use `ExecuteMultipleRequest`, `ExecuteMultipleResponse`, and the typed create/update/delete request wrappers.

See [doc/batch.md](doc/batch.md).

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
