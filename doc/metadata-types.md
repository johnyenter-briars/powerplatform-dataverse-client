# Metadata Types

The crate exposes metadata models for schema-driven Dataverse scenarios such as intellisense, schema browsing, and dynamic query shaping.

Microsoft Learn background:

- [Query table definitions using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/query-metadata-web-api)

## Public API

- `AttributeTypeName`
- `EntityAttribute`
- `EntityDefinition`
- `EntityRelationship`

## How They Map

- `EntityDefinition` models table-level metadata such as logical name, schema name, entity set name, and primary id attribute.
- `EntityAttribute` models attribute-level metadata returned from the Dataverse metadata endpoints.
- `AttributeTypeName` captures the nested `{"Value": "..."}` payload Dataverse uses for specific attribute-type names.
- `EntityRelationship` normalizes Dataverse relationship metadata into a single Rust shape across different relationship families.

## Service Client Methods

- `ServiceClient::list_entity_definitions(&self) -> Result<Vec<EntityDefinition>, String>`
- `ServiceClient::list_entity_attributes(&self, logical_name: &str) -> Result<Vec<EntityAttribute>, String>`
- `ServiceClient::list_entity_relationships(&self, logical_name: &str) -> Result<Vec<EntityRelationship>, String>`

## Related Page

See [metadata.md](metadata.md) for higher-level usage notes and sample flow.
