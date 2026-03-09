use std::collections::HashMap;

use chrono::{DateTime, NaiveDate};
use log::warn;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::dataverse::entity::{
    Attribute, DateTimeValue, Entity, EntityReference, MoneyValue, MultiSelectOptionSetValue,
    OptionSetValue, Value as RowValue,
};

const FORMATTED_VALUE_SUFFIX: &str = "@OData.Community.Display.V1.FormattedValue";
const LOOKUP_LOGICAL_NAME_SUFFIX: &str = "@Microsoft.Dynamics.CRM.lookuplogicalname";

#[derive(Default)]
struct RecordAnnotations {
    formatted_values: HashMap<String, String>,
    lookup_logical_names: HashMap<String, String>,
}

impl RecordAnnotations {
    fn collect(record: &Map<String, Value>) -> Self {
        let mut annotations = Self::default();

        for (key, value) in record {
            if let Some(base) = key.strip_suffix(FORMATTED_VALUE_SUFFIX) {
                if let Some(formatted) = value.as_str() {
                    annotations
                        .formatted_values
                        .insert(base.to_string(), formatted.to_string());
                }
                continue;
            }

            if let Some(base) = key.strip_suffix(LOOKUP_LOGICAL_NAME_SUFFIX) {
                if let Some(logical_name) = value.as_str() {
                    annotations
                        .lookup_logical_names
                        .insert(base.to_string(), logical_name.to_string());
                }
            }
        }

        annotations
    }

    fn formatted_value(&self, key: &str) -> Option<&str> {
        self.formatted_values.get(key).map(String::as_str)
    }

    fn lookup_logical_name(&self, key: &str) -> Option<&str> {
        self.lookup_logical_names.get(key).map(String::as_str)
    }
}

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
        let record = record_value
            .as_object()
            .ok_or_else(|| "Invalid response from Dataverse".to_string())?;
        entities.push(parse_record(record)?);
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
fn parse_record(record: &Map<String, Value>) -> Result<Entity, std::string::String> {
    let annotations = RecordAnnotations::collect(record);
    let mut attributes: HashMap<Attribute, RowValue> = HashMap::new();

    for (key, value) in record {
        if key.contains('@') {
            continue;
        }

        let implemented = add_attribute(&mut attributes, key, value, &annotations)?;
        if !implemented {
            warn!("Unsupported Dataverse key: {}", key);
        }
    }

    Ok(Entity { attributes })
}

fn add_attribute(
    attributes: &mut HashMap<Attribute, RowValue>,
    key: &str,
    value: &Value,
    annotations: &RecordAnnotations,
) -> Result<bool, std::string::String> {
    if value.is_null() {
        attributes.insert(key.to_string(), RowValue::Null);
        return Ok(true);
    }

    if let Some(reference) = parse_entity_reference(key, value, annotations)? {
        attributes.insert(key.to_string(), RowValue::EntityReference(reference));
        return Ok(true);
    }

    if let Some(value) = parse_multi_select_option_set(value, annotations.formatted_value(key)) {
        attributes.insert(key.to_string(), RowValue::MultiSelectOptionSet(value));
        return Ok(true);
    }

    if let Some(value) = parse_datetime_value(value, annotations.formatted_value(key)) {
        attributes.insert(key.to_string(), RowValue::DateTime(value));
        return Ok(true);
    }

    if let Some(value) = parse_money_value(value, annotations.formatted_value(key)) {
        attributes.insert(key.to_string(), RowValue::Money(value));
        return Ok(true);
    }

    if let Some(value) = parse_option_set_value(value, annotations.formatted_value(key)) {
        attributes.insert(key.to_string(), RowValue::OptionSet(value));
        return Ok(true);
    }

    if value.is_i64() {
        let i = value
            .as_i64()
            .ok_or_else(|| format!("Unable to parse dataverse value: {:?}", value))?;
        attributes.insert(key.to_string(), RowValue::Int(i));
        return Ok(true);
    }

    if value.is_u64() {
        let i = value
            .as_u64()
            .ok_or_else(|| format!("Unable to parse dataverse value: {:?}", value))?;
        if let Ok(as_i64) = i64::try_from(i) {
            attributes.insert(key.to_string(), RowValue::Int(as_i64));
        } else {
            attributes.insert(key.to_string(), RowValue::Float(i as f64));
        }
        return Ok(true);
    }

    if value.is_f64() {
        let f = value
            .as_f64()
            .ok_or_else(|| format!("Unable to parse dataverse value: {:?}", value))?;
        attributes.insert(key.to_string(), RowValue::Float(f));
        return Ok(true);
    }

    if value.is_string() {
        let s = value
            .as_str()
            .ok_or_else(|| format!("Unable to parse dataverse value: {:?}", value))?;
        attributes.insert(key.to_string(), RowValue::String(s.to_string()));
        return Ok(true);
    }

    if value.is_boolean() {
        let b = value
            .as_bool()
            .ok_or_else(|| format!("Unable to parse dataverse value: {:?}", value))?;
        attributes.insert(key.to_string(), RowValue::Boolean(b));
        return Ok(true);
    }

    Ok(true)
}

fn parse_entity_reference(
    key: &str,
    value: &Value,
    annotations: &RecordAnnotations,
) -> Result<Option<EntityReference>, std::string::String> {
    if !looks_like_lookup_property(key) {
        return Ok(None);
    }

    let raw = value
        .as_str()
        .ok_or_else(|| format!("Unable to parse dataverse lookup value: {:?}", value))?;
    let trimmed = raw.trim_matches(|ch| ch == '{' || ch == '}');
    let id = Uuid::parse_str(trimmed)
        .map_err(|error| format!("Unable to parse Dataverse lookup GUID '{trimmed}': {error}"))?;

    Ok(Some(EntityReference {
        id,
        logical_name: annotations
            .lookup_logical_name(key)
            .unwrap_or_default()
            .to_string(),
        name: annotations.formatted_value(key).map(str::to_string),
    }))
}

fn parse_option_set_value(value: &Value, formatted: Option<&str>) -> Option<OptionSetValue> {
    let formatted = formatted?;
    if looks_like_money_formatted(formatted) {
        return None;
    }

    if let Some(raw) = value.as_i64() {
        return Some(OptionSetValue {
            value: raw,
            name: Some(formatted.to_string()),
        });
    }

    let raw = value.as_u64()?;
    let raw = i64::try_from(raw).ok()?;
    Some(OptionSetValue {
        value: raw,
        name: Some(formatted.to_string()),
    })
}

fn parse_multi_select_option_set(
    value: &Value,
    formatted: Option<&str>,
) -> Option<MultiSelectOptionSetValue> {
    let raw = value.as_str()?;
    let formatted = formatted?;

    let mut values: Vec<i64> = Vec::new();
    for part in raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        values.push(trimmed.parse::<i64>().ok()?);
    }

    if values.is_empty() {
        return None;
    }

    let names = formatted
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<String>>();

    Some(MultiSelectOptionSetValue { values, names })
}

fn parse_money_value(value: &Value, formatted: Option<&str>) -> Option<MoneyValue> {
    let formatted = formatted?;
    if !looks_like_money_formatted(formatted) {
        return None;
    }

    Some(MoneyValue {
        value: numeric_value(value)?,
        formatted: Some(formatted.to_string()),
    })
}

fn parse_datetime_value(value: &Value, formatted: Option<&str>) -> Option<DateTimeValue> {
    let raw = value.as_str()?;
    if formatted.is_none() || !looks_like_datetime(raw) {
        return None;
    }

    Some(DateTimeValue {
        value: raw.to_string(),
        formatted: formatted.map(str::to_string),
    })
}

fn numeric_value(value: &Value) -> Option<f64> {
    if let Some(value) = value.as_f64() {
        return Some(value);
    }
    if let Some(value) = value.as_i64() {
        return Some(value as f64);
    }
    value.as_u64().map(|value| value as f64)
}

fn looks_like_lookup_property(key: &str) -> bool {
    let tail = key.rsplit('.').next().unwrap_or(key);
    tail.starts_with('_') && tail.ends_with("_value") && tail.len() > "_value".len() + 1
}

fn looks_like_datetime(raw: &str) -> bool {
    DateTime::parse_from_rfc3339(raw).is_ok() || NaiveDate::parse_from_str(raw, "%Y-%m-%d").is_ok()
}

fn looks_like_money_formatted(formatted: &str) -> bool {
    formatted.chars().any(|ch| {
        matches!(
            ch,
            '$' | '£' | '€' | '¥' | '₹' | '₩' | '₽' | '₺' | '₴' | '₱' | '₫' | '₪' | '₦'
        )
    }) || formatted
        .split_whitespace()
        .any(|part| part.chars().all(|ch| ch.is_ascii_alphabetic()) && part.len() == 3)
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::parse_entities_from_response;
    use crate::dataverse::entity::{
        DateTimeValue, EntityReference, MoneyValue, MultiSelectOptionSetValue, OptionSetValue,
        Value,
    };

    #[test]
    fn parses_typed_dataverse_values_from_annotations() {
        let json = json!({
            "value": [{
                "_customerid_value": "00000000-0000-0000-0000-000000000001",
                "_customerid_value@OData.Community.Display.V1.FormattedValue": "Contoso",
                "_customerid_value@Microsoft.Dynamics.CRM.lookuplogicalname": "account",
                "statuscode": 1,
                "statuscode@OData.Community.Display.V1.FormattedValue": "Active",
                "sample_multiselect": "100000000,100000001",
                "sample_multiselect@OData.Community.Display.V1.FormattedValue": "Red; Blue",
                "revenue": 45.5,
                "revenue@OData.Community.Display.V1.FormattedValue": "$45.50",
                "createdon": "2026-03-08T15:30:00Z",
                "createdon@OData.Community.Display.V1.FormattedValue": "3/8/2026 3:30 PM"
            }]
        });

        let entities = parse_entities_from_response(&json).expect("entities");
        let entity = &entities[0];

        assert_eq!(
            entity.attributes.get("_customerid_value"),
            Some(&Value::EntityReference(EntityReference {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").expect("uuid"),
                logical_name: "account".to_string(),
                name: Some("Contoso".to_string()),
            }))
        );
        assert_eq!(
            entity.attributes.get("statuscode"),
            Some(&Value::OptionSet(OptionSetValue {
                value: 1,
                name: Some("Active".to_string()),
            }))
        );
        assert_eq!(
            entity.attributes.get("sample_multiselect"),
            Some(&Value::MultiSelectOptionSet(MultiSelectOptionSetValue {
                values: vec![100000000, 100000001],
                names: vec!["Red".to_string(), "Blue".to_string()],
            }))
        );
        assert_eq!(
            entity.attributes.get("revenue"),
            Some(&Value::Money(MoneyValue {
                value: 45.5,
                formatted: Some("$45.50".to_string()),
            }))
        );
        assert_eq!(
            entity.attributes.get("createdon"),
            Some(&Value::DateTime(DateTimeValue {
                value: "2026-03-08T15:30:00Z".to_string(),
                formatted: Some("3/8/2026 3:30 PM".to_string()),
            }))
        );
    }
}
