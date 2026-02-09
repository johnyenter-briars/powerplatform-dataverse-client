use std::collections::HashMap;

use serde_json::Value;

use crate::dataverse::entity::Value::{Boolean, Float, Int, Null, String};
use crate::dataverse::entity::{Attribute, Entity, Value as RowValue};

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
pub(crate) fn parse_entities_from_response(json: &Value) -> Result<Vec<Entity>, std::string::String> {
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
        let mut entity = Entity::new();

        let record = record_value
            .as_object()
            .ok_or_else(|| "Invalid response from Dataverse".to_string())?;

        for (key, value) in record {
            let implemented = add_attribute(&mut entity.attributes, key, value)
                .map_err(|_| "Invalid response from Dataverse".to_string())?;

            if !implemented {
                println!("Key: {}, implemented: {:?}", key, implemented);
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
