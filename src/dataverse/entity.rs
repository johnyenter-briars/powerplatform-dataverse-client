use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Dataverse lookup or polymorphic lookup reference.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EntityReference {
    /// Referenced record ID.
    pub id: Uuid,
    /// Referenced entity logical name.
    pub logical_name: String,
    /// Display name, when Dataverse returns a formatted value annotation.
    pub name: Option<String>,
}

/// Dataverse option set value and label.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct OptionSetValue {
    /// Underlying integer value.
    pub value: i64,
    /// Display label, when Dataverse returns a formatted value annotation.
    pub name: Option<String>,
}

/// Dataverse multi-select option set values and labels.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MultiSelectOptionSetValue {
    /// Underlying integer values.
    pub values: Vec<i64>,
    /// Display labels returned by Dataverse.
    pub names: Vec<String>,
}

/// Dataverse money value and formatted display.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MoneyValue {
    /// Underlying numeric value.
    pub value: f64,
    /// Formatted display string, when present.
    pub formatted: Option<String>,
}

/// Dataverse date/time value and formatted display.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DateTimeValue {
    /// Raw ISO-8601 value returned by Dataverse.
    pub value: String,
    /// Formatted display string, when present.
    pub formatted: Option<String>,
}

/// Represents a Dataverse attribute value.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
    /// Lookup or polymorphic lookup reference.
    EntityReference(EntityReference),
    /// Option set value and label.
    OptionSet(OptionSetValue),
    /// Multi-select option set values and labels.
    MultiSelectOptionSet(MultiSelectOptionSetValue),
    /// Money value and formatted display.
    Money(MoneyValue),
    /// Date/time value and formatted display.
    DateTime(DateTimeValue),
    /// Null value.
    Null,
}

/// Attribute logical name.
pub type Attribute = String;

/// Dataverse entity record with attribute values.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
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
