use std::collections::HashMap;

use log::warn;
use serde_json::Value;

use crate::dataverse::entity::Value::{
    Boolean, EntityReference as EntityRefValue, Float, Int, Null, String,
};
use crate::dataverse::entity::{Attribute, Entity, EntityReference, Value as RowValue};
use uuid::Uuid;

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

    for record_value in response_array {
        let mut entity = Entity::default();

        let record = record_value
            .as_object()
            .ok_or_else(|| "Invalid response from Dataverse".to_string())?;

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

            let implemented = add_attribute(&mut entity.attributes, key, value)
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
) -> Result<bool, std::string::String> {
    if value.is_null() {
        attributes.insert(key.to_string(), Null);
        return Ok(true);
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
