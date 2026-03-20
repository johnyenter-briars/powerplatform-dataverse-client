use std::collections::HashMap;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use log::warn;
use rust_decimal::Decimal;
use serde_json::Value;

use crate::dataverse::entity::Value::{
    Boolean, DateTime as DateTimeValue, Decimal as DecimalValue,
    EntityReference as EntityRefValue, Float, Guid as GuidValue, Int, Money as MoneyValue, Null,
    OptionSetValue as OptionSetSingle, OptionSetValueCollection as OptionSetMany, String,
};
use crate::dataverse::entity::{
    Attribute, Entity, EntityReference, Money, OptionSetValue, OptionSetValueCollection,
    Value as RowValue,
};
use crate::dataverse::entityattribute::EntityAttribute;
use uuid::Uuid;

const FORMATTED_VALUE_SUFFIX: &str = "@OData.Community.Display.V1.FormattedValue";

/// Determine if a Dataverse response indicates more records.
pub(crate) fn parse_more_records(json: &Value) -> bool {
    match json.get("@Microsoft.Dynamics.CRM.morerecords") {
        Some(Value::Bool(value)) => *value,
        Some(Value::String(value)) => value.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

/// Extract the paging cookie from a Dataverse response.
pub(crate) fn extract_paging_cookie(json: &Value) -> Option<std::string::String> {
    let cookie_element = json
        .get("@Microsoft.Dynamics.CRM.fetchxmlpagingcookie")
        .and_then(|value| value.as_str())?;
    let key = "pagingcookie=\"";
    let start = cookie_element.find(key)? + key.len();
    let end = cookie_element[start..].find('"')? + start;
    let encoded = &cookie_element[start..end];
    let decoded_once = urlencoding::decode(encoded).ok()?.into_owned();
    let decoded_twice = urlencoding::decode(&decoded_once).ok()?.into_owned();
    Some(decoded_twice)
}

/// Parse entities from a Dataverse list response.
pub(crate) fn parse_entities_from_response(
    json: &Value,
    entity_set: &str,
    primary_id_attribute: Option<&str>,
    entity_attributes: Option<&HashMap<std::string::String, EntityAttribute>>,
) -> Result<Vec<Entity>, std::string::String> {
    let response_object = json
        .as_object()
        .ok_or_else(|| "Invalid response from Dataverse".to_string())?;

    let response_array = response_object
        .get("value")
        .ok_or_else(|| "Invalid response from Dataverse".to_string())?
        .as_array()
        .ok_or_else(|| "Invalid response from Dataverse".to_string())?;

    let mut entities: Vec<Entity> = vec![];
    let logical_name = infer_logical_name(entity_set);
    let primary_id_key = primary_id_attribute
        .map(|value| value.to_string())
        .unwrap_or_else(|| format!("{}id", logical_name));

    for record_value in response_array {
        let record = record_value
            .as_object()
            .ok_or_else(|| "Invalid response from Dataverse".to_string())?;

        // NOTE: Convention-based primary id without a metadata call.
        // If missing, we fail fast so the caller can correct it.
        let id_value = record
            .get(&primary_id_key)
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                format!(
                    "Primary id '{}' not found for entity set '{}'",
                    primary_id_key, entity_set
                )
            })?;
        let id =
            Uuid::parse_str(id_value).map_err(|_| "Invalid response from Dataverse".to_string())?;

        let name = record
            .get("name")
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        let mut entity = Entity::new(id, &logical_name, name);

        let mut lookup_keys: Vec<(std::string::String, std::string::String)> = Vec::new();

        for (key, value) in record {
            if key.contains('@') {
                continue;
            }

            if let Some(base) = lookup_base_attribute(key) {
                let id = value
                    .as_str()
                    .map(|value| value.to_string())
                    .unwrap_or_default();
                lookup_keys.push((key.to_string(), base.clone()));

                if id.is_empty() {
                    entity.attributes.insert(base, Null);
                }
                continue;
            }

            let implemented = add_attribute(
                &mut entity.attributes,
                key,
                value,
                entity_attributes.and_then(|attributes| {
                    attributes.get(&normalize_attribute_name(key))
                }),
            )
                .map_err(|_| "Invalid response from Dataverse".to_string())?;

            if !implemented {
                warn!("Unsupported Dataverse key: {}", key);
            }
        }

        for (raw_key, base) in lookup_keys {
            if entity.attributes.contains_key(&base) {
                continue;
            }

            let Some(id_value) = record.get(&raw_key).and_then(|value| value.as_str()) else {
                entity.attributes.insert(base, Null);
                continue;
            };

            let logical_key = format!("{raw_key}@Microsoft.Dynamics.CRM.lookuplogicalname");
            let formatted_key = format!("{raw_key}@OData.Community.Display.V1.FormattedValue");

            let logical_name = record
                .get(&logical_key)
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());

            let name = record
                .get(&formatted_key)
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());

            if let Some(name) = name.as_ref() {
                entity
                    .attributes
                    .insert(format!("{base}name"), String(name.clone()));
            }

            if let Some(logical_name) = logical_name {
                let id = Uuid::parse_str(id_value)
                    .map_err(|_| "Invalid response from Dataverse".to_string())?;
                entity.attributes.insert(
                    base,
                    EntityRefValue(EntityReference {
                        id,
                        logical_name,
                        name,
                    }),
                );
            } else {
                warn!("Lookup logical name missing for key: {}", raw_key);
                entity.attributes.insert(base, String(id_value.to_string()));
            }
        }

        apply_lookup_attribute_annotations(&mut entity.attributes, record);
        apply_formatted_value_names(&mut entity.attributes, record);

        entities.push(entity);
    }

    Ok(entities)
}

/// Count the number of records in a Dataverse list response.
pub(crate) fn parse_record_count_from_response(json: &Value) -> Result<usize, std::string::String> {
    let response_object = json
        .as_object()
        .ok_or_else(|| "Invalid response from Dataverse".to_string())?;

    let response_array = response_object
        .get("value")
        .ok_or_else(|| "Invalid response from Dataverse".to_string())?
        .as_array()
        .ok_or_else(|| "Invalid response from Dataverse".to_string())?;

    Ok(response_array.len())
}

/// Convert a JSON value into a Dataverse attribute value.
fn add_attribute(
    attributes: &mut HashMap<Attribute, RowValue>,
    key: &str,
    value: &Value,
    attribute: Option<&EntityAttribute>,
) -> Result<bool, std::string::String> {
    if value.is_null() {
        attributes.insert(key.to_string(), Null);
        return Ok(true);
    }

    if let Some(attribute_type) = attribute_type_key(attribute) {
        if let Some(parsed) = parse_typed_attribute_value(value, attribute_type)? {
            attributes.insert(key.to_string(), parsed);
            return Ok(true);
        }
    }

    if value.is_i64() {
        let i = value
            .as_i64()
            .ok_or(format!("Unable to parse dataverse value: {:?}", value))?;
        attributes.insert(key.to_string(), Int(i));
        return Ok(true);
    }

    if value.is_u64() {
        let i = value
            .as_u64()
            .ok_or(format!("Unable to parse dataverse value: {:?}", value))?;
        if let Ok(as_i64) = i64::try_from(i) {
            attributes.insert(key.to_string(), Int(as_i64));
        } else {
            attributes.insert(key.to_string(), Float(i as f64));
        }
        return Ok(true);
    }

    if value.is_f64() {
        let f = value
            .as_f64()
            .ok_or(format!("Unable to parse dataverse value: {:?}", value))?;
        attributes.insert(key.to_string(), Float(f));
        return Ok(true);
    }

    if value.is_string() {
        let s = value
            .as_str()
            .ok_or(format!("Unable to parse dataverse value: {:?}", value))?;
        attributes.insert(key.to_string(), String(s.to_string()));
        return Ok(true);
    }

    if value.is_boolean() {
        let b = value
            .as_bool()
            .ok_or(format!("Unable to parse dataverse value: {:?}", value))?;
        attributes.insert(key.to_string(), Boolean(b));
        return Ok(true);
    }

    Ok(true)
}

fn parse_typed_attribute_value(
    value: &Value,
    attribute_type: &str,
) -> Result<Option<RowValue>, std::string::String> {
    match attribute_type {
        "BigInt" | "BigIntType" => Ok(parse_i64_value(value).map(Int)),
        "Boolean" | "BooleanType" => Ok(parse_bool_value(value).map(Boolean)),
        "DateTime" | "DateTimeType" => Ok(parse_datetime_value(value).map(DateTimeValue)),
        "Decimal" | "DecimalType" => Ok(parse_decimal_value(value).map(DecimalValue)),
        "Double" | "DoubleType" => Ok(parse_f64_value(value).map(Float)),
        "Integer" | "IntegerType" => Ok(parse_i64_value(value).map(Int)),
        "Guid" | "Uniqueidentifier" | "UniqueidentifierType" => {
            Ok(parse_guid_value(value).map(GuidValue))
        }
        "Money" | "MoneyType" => Ok(parse_decimal_value(value).map(|value| {
            MoneyValue(Money { value })
        })),
        "Picklist" | "PicklistType" | "State" | "StateType" | "Status" | "StatusType" => {
            Ok(parse_i32_value(value).map(|value| {
                OptionSetSingle(OptionSetValue { value, name: None })
            }))
        }
        "MultiSelectPicklist" | "MultiSelectPicklistType" => {
            Ok(parse_multi_select_value(value).map(|values| {
                OptionSetMany(OptionSetValueCollection { values })
            }))
        }
        "Customer"
        | "CustomerType"
        | "Lookup"
        | "LookupType"
        | "Owner"
        | "OwnerType"
        | "PartyList"
        | "PartyListType" => Ok(None),
        "String"
        | "StringType"
        | "Memo"
        | "MemoType"
        | "EntityName"
        | "EntityNameType"
        | "Image"
        | "ImageType"
        | "File"
        | "FileType" => Ok(value.as_str().map(|value| String(value.to_string()))),
        _ => Ok(None),
    }
}

fn attribute_type_key(attribute: Option<&EntityAttribute>) -> Option<&str> {
    attribute
        .and_then(|attribute| {
            attribute
                .attribute_type_name
                .as_ref()
                .and_then(|value| value.value.as_deref())
                .or(attribute.attribute_type.as_deref())
        })
}

fn normalize_attribute_name(value: &str) -> std::string::String {
    value.to_ascii_lowercase()
}

fn parse_i64_value(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
}

fn parse_i32_value(value: &Value) -> Option<i32> {
    parse_i64_value(value).and_then(|value| i32::try_from(value).ok())
}

fn parse_f64_value(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|value| value as f64))
        .or_else(|| value.as_u64().map(|value| value as f64))
}

fn parse_bool_value(value: &Value) -> Option<bool> {
    value.as_bool()
}

fn parse_decimal_value(value: &Value) -> Option<Decimal> {
    match value {
        Value::Number(number) => Decimal::from_str(&number.to_string()).ok(),
        Value::String(value) => Decimal::from_str(value).ok(),
        _ => None,
    }
}

fn parse_datetime_value(value: &Value) -> Option<DateTime<Utc>> {
    value
        .as_str()
        .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
        .map(|value| value.with_timezone(&Utc))
}

fn parse_guid_value(value: &Value) -> Option<Uuid> {
    value
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn parse_multi_select_value(value: &Value) -> Option<Vec<i32>> {
    match value {
        Value::String(value) => {
            let parsed: Vec<i32> = value
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .filter_map(|value| value.parse::<i32>().ok())
                .collect();
            Some(parsed)
        }
        Value::Array(values) => Some(
            values
                .iter()
                .filter_map(parse_i32_value)
                .collect(),
        ),
        _ => None,
    }
}

fn lookup_base_attribute(key: &str) -> Option<std::string::String> {
    if !key.starts_with('_') || !key.ends_with("_value") {
        return None;
    }

    let trimmed = &key[1..key.len() - "_value".len()];
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

fn apply_formatted_value_names(
    attributes: &mut HashMap<Attribute, RowValue>,
    record: &serde_json::Map<std::string::String, Value>,
) {
    if !record.keys().any(|key| key.ends_with(FORMATTED_VALUE_SUFFIX)) {
        return;
    }

    for (key, value) in record {
        let Some(base_key) = key.strip_suffix(FORMATTED_VALUE_SUFFIX) else {
            continue;
        };

        let Some(formatted) = value.as_str() else {
            continue;
        };

        let Some(attribute) = attributes.get_mut(base_key) else {
            continue;
        };

        if let OptionSetSingle(option) = attribute {
            option.name = Some(formatted.to_string());
        }
    }
}

fn apply_lookup_attribute_annotations(
    attributes: &mut HashMap<Attribute, RowValue>,
    record: &serde_json::Map<std::string::String, Value>,
) {
    for (key, value) in record {
        let Some(base_key) = key.strip_suffix("@Microsoft.Dynamics.CRM.lookuplogicalname") else {
            continue;
        };

        if base_key.starts_with('_') {
            continue;
        }

        let Some(logical_name) = value.as_str() else {
            continue;
        };

        let Some(attribute) = attributes.get_mut(base_key) else {
            continue;
        };

        let RowValue::String(id_value) = attribute else {
            continue;
        };

        let Ok(id) = Uuid::parse_str(id_value) else {
            continue;
        };

        let formatted_key = format!("{base_key}{FORMATTED_VALUE_SUFFIX}");
        let name = record
            .get(&formatted_key)
            .and_then(|value| value.as_str())
            .map(|value| value.to_string());

        *attribute = EntityRefValue(EntityReference {
            id,
            logical_name: logical_name.to_string(),
            name,
        });
    }
}

fn infer_logical_name(entity_set: &str) -> std::string::String {
    let normalized = entity_set.trim().to_ascii_lowercase();

    if normalized.ends_with("ies") && normalized.len() > 3 {
        return format!("{}y", &normalized[..normalized.len() - 3]);
    }

    if ends_with_any(&normalized, &["ses", "xes", "zes", "ches", "shes"]) && normalized.len() > 2 {
        return normalized[..normalized.len() - 2].to_string();
    }

    if normalized.ends_with('s')
        && !normalized.ends_with("ss")
        && !normalized.ends_with("us")
        && !normalized.ends_with("is")
        && normalized.len() > 1
    {
        return normalized[..normalized.len() - 1].to_string();
    }

    normalized
}

fn ends_with_any(name: &str, suffixes: &[&str]) -> bool {
    suffixes.iter().any(|suffix| name.ends_with(suffix))
}
