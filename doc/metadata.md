# Metadata

The client supports Dataverse metadata retrieval for entity definitions and entity attributes.

## Supported Methods

- `list_entity_definitions`
- `list_entity_attributes`

## Notes

- Entity definitions are retrieved from the Dataverse metadata endpoints.
- Attribute listing is filtered to readable OData-compatible fields.

## Sample Scenario

See [`samples/v1-features/src/scenarios/metadata.rs`](../samples/v1-features/src/scenarios/metadata.rs).
