use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
}

/// Attribute logical name.
pub type Attribute = String;

/// Dataverse entity record with attribute values.
#[derive(Debug, Serialize, Deserialize)]
pub struct Entity {
    /// Attribute map keyed by logical names.
    pub attributes: HashMap<Attribute, Value>,
}

impl Entity {
    /// Create a new empty entity.
    pub fn new() -> Self {
        Entity {
            attributes: HashMap::new(),
        }
    }
}
