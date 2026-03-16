# Metadata

The client supports Dataverse metadata retrieval for entity definitions, entity attributes, and entity relationships.

## Supported Methods

- `list_entity_definitions`
- `list_entity_attributes`
- `list_entity_relationships`

## Notes

- Entity definitions are retrieved from the Dataverse metadata endpoints.
- Attribute listing is filtered to readable OData-compatible fields.
- Relationship listing returns many-to-one, one-to-many, and many-to-many metadata for the selected entity.

## Example

```rust
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;
use powerplatform_dataverse_client::LogLevel;

#[tokio::main]
async fn main() -> Result<(), String> {
    let client = ServiceClient::new("YOUR_CONNECTION_STRING", LogLevel::Information).await?;

    let definitions = client.list_entity_definitions().await?;
    println!("Entity definitions: {}", definitions.len());

    let attributes = client.list_entity_attributes("account").await?;
    println!("Account attributes: {}", attributes.len());

    let relationships = client.list_entity_relationships("account").await?;
    println!("Account relationships: {}", relationships.len());

    Ok(())
}
```

## Sample Scenario

See [`samples/v1-features/src/scenarios/metadata.rs`](../samples/v1-features/src/scenarios/metadata.rs).
