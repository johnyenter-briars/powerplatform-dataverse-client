use serde::{Deserialize, Serialize};

/// Dataverse attribute metadata.
#[derive(Debug, Serialize, Deserialize)]
pub struct EntityAttribute {
    /// Logical name of the attribute.
    #[serde(rename = "LogicalName")]
    pub logical_name: String,
    /// Schema name of the attribute.
    #[serde(rename = "SchemaName")]
    pub schema_name: String,
    /// Attribute type name.
    #[serde(rename = "AttributeType")]
    pub attribute_type: Option<String>,
    /// True if the attribute is custom.
    #[serde(rename = "IsCustomAttribute")]
    pub is_custom_attribute: Option<bool>,
    /// True if the attribute is valid for OData.
    #[serde(rename = "IsValidODataAttribute")]
    pub is_valid_odata_attribute: Option<bool>,
    /// True if the attribute is valid for read operations.
    #[serde(rename = "IsValidForRead")]
    pub is_valid_for_read: Option<bool>,
}
