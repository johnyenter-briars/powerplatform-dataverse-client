use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Dataverse entity definition metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityDefinition {
    /// OData context metadata.
    #[serde(rename = "@odata.context")]
    pub odata_context: Option<String>,
    /// Logical name of the entity.
    #[serde(rename = "LogicalName")]
    pub logical_name: String,
    /// Schema name of the entity.
    #[serde(rename = "SchemaName")]
    pub schema_name: String,
    /// Display name payload.
    #[serde(rename = "DisplayName")]
    pub display_name: Option<Value>,
    /// Entity set (collection) name.
    #[serde(rename = "EntitySetName")]
    pub entity_set_name: String,
    /// True if the entity is custom.
    #[serde(rename = "IsCustomEntity")]
    pub is_custom_entity: bool,
    /// Primary ID attribute logical name.
    #[serde(rename = "PrimaryIdAttribute")]
    pub primary_id_attribute: Option<String>,
    /// Additional fields returned by the API.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
