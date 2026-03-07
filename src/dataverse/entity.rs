use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a Dataverse attribute value.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Value {
    /// Signed 64-bit integer.
    Int(i64),
    /// Floating point value.
    Float(f64),
    /// String value.
    String(String),
    /// Boolean value.
    Boolean(bool),
    /// Null value.
    Null,
    /// Entity reference value (lookup).
    EntityReference(EntityReference),
}

/// Reference to another Dataverse entity.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EntityReference {
    /// Primary ID for the referenced entity.
    pub id: Uuid,
    /// Logical name for the referenced entity.
    pub logical_name: String,
    /// Primary name for the referenced entity, when provided.
    pub name: Option<String>,
}

/// Attribute logical name.
pub type Attribute = String;

/// Dataverse entity record with attribute values.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Entity {
    /// Attribute map keyed by logical names.
    pub attributes: HashMap<Attribute, Value>,
}
