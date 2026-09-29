use std::collections::HashMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
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
    /// Exact decimal value.
    Decimal(Decimal),
    /// String value.
    String(String),
    /// Boolean value.
    Boolean(bool),
    /// Date/time value.
    DateTime(DateTime<Utc>),
    /// GUID value.
    Guid(Uuid),
    /// Money value.
    Money(Money),
    /// Single choice/status/state value.
    OptionSetValue(OptionSetValue),
    /// Multi-select choice value.
    OptionSetValueCollection(OptionSetValueCollection),
    /// Null value.
    Null,
    /// Entity reference value (lookup).
    EntityReference(EntityReference),
}

/// Dataverse money value.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Money {
    /// Monetary amount.
    pub value: Decimal,
}

/// Dataverse single option-set value.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptionSetValue {
    /// Numeric option value.
    pub value: i32,
    /// Display label for the option, when provided.
    pub name: Option<String>,
}

/// Dataverse multi-select option-set value.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OptionSetValueCollection {
    /// Numeric option values.
    pub values: Vec<i32>,
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
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    /// Primary ID for the entity record.
    pub id: Uuid,
    /// Logical name for the entity.
    pub logical_name: String,
    /// Primary name for the entity record, when provided.
    pub name: Option<String>,
    /// Attribute map keyed by logical names.
    pub attributes: HashMap<Attribute, Value>,
}

impl Entity {
    /// Create a new entity with the provided identity fields.
    pub fn new(id: Uuid, logical_name: impl Into<String>, name: Option<String>) -> Self {
        Self {
            id,
            logical_name: logical_name.into(),
            name,
            attributes: HashMap::new(),
        }
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            logical_name: String::new(),
            name: None,
            attributes: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Entity;
    use uuid::Uuid;

    #[test]
    fn new_entity_starts_with_empty_attribute_map() {
        let id = Uuid::new_v4();
        let entity = Entity::new(id, "account", Some("Acme".to_string()));

        assert_eq!(entity.id, id);
        assert_eq!(entity.logical_name, "account");
        assert_eq!(entity.name.as_deref(), Some("Acme"));
        assert!(entity.attributes.is_empty());
    }

    #[test]
    fn default_entity_uses_nil_identity() {
        let entity = Entity::default();

        assert_eq!(entity.id, Uuid::nil());
        assert!(entity.logical_name.is_empty());
        assert!(entity.name.is_none());
        assert!(entity.attributes.is_empty());
    }
}
