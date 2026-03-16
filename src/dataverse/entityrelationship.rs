use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Dataverse relationship metadata normalized across relationship types.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityRelationship {
    #[serde(rename = "SchemaName")]
    pub schema_name: String,
    #[serde(rename = "RelationshipType")]
    pub relationship_type: String,
    #[serde(rename = "ReferencedEntity")]
    pub referenced_entity: Option<String>,
    #[serde(rename = "ReferencedAttribute")]
    pub referenced_attribute: Option<String>,
    #[serde(rename = "ReferencingEntity")]
    pub referencing_entity: Option<String>,
    #[serde(rename = "ReferencingAttribute")]
    pub referencing_attribute: Option<String>,
    #[serde(rename = "IntersectEntityName")]
    pub intersect_entity_name: Option<String>,
    #[serde(rename = "IsCustomRelationship")]
    pub is_custom_relationship: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
