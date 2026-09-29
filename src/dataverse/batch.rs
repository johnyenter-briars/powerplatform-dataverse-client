use std::collections::HashMap;

use reqwest::header::CONTENT_TYPE;
use serde_json::{Map, Number, Value as JsonValue};
use uuid::Uuid;

use crate::dataverse::entity::{
    Entity, EntityReference, OptionSetValueCollection, Value as DataverseValue,
};
use crate::dataverse::requestparameters::RequestParameters;

const HEADER_SEPARATOR: &str = "\r\n\r\n";

#[derive(Debug, Clone, Default)]
pub struct ExecuteMultipleSettings {
    /// Continue processing later requests when an earlier request fails.
    pub continue_on_error: bool,
    /// Include per-request success payloads in the batch response when Dataverse provides them.
    pub return_responses: bool,
}

/// Batch request payload for Dataverse `ExecuteMultiple`-style operations.
#[derive(Debug, Clone, Default)]
pub struct ExecuteMultipleRequest {
    /// Batch execution behavior flags.
    pub settings: ExecuteMultipleSettings,
    /// Individual create, update, or delete operations to include in the batch.
    pub requests: Vec<OrganizationRequest>,
}

/// Batch response returned from Dataverse after executing multiple operations.
#[derive(Debug, Clone, Default)]
pub struct ExecuteMultipleResponse {
    /// Per-request outcomes in the same order as the submitted requests.
    pub responses: Vec<ExecuteMultipleResponseItem>,
}

/// Result for a single request within an `ExecuteMultipleResponse`.
#[derive(Debug, Clone)]
pub struct ExecuteMultipleResponseItem {
    /// Zero-based index of the original request in the submitted batch.
    pub request_index: usize,
    /// Successful response payload, when `return_responses` was enabled and the request succeeded.
    pub response: Option<OrganizationResponse>,
    /// Fault details for failed requests.
    pub fault: Option<OrganizationServiceFault>,
}

/// Error payload for a failed Dataverse batch item.
#[derive(Debug, Clone)]
pub struct OrganizationServiceFault {
    /// HTTP status code returned for the failed item.
    pub status_code: u16,
    /// Dataverse fault code when present in the response body.
    pub code: Option<String>,
    /// Human-readable fault message.
    pub message: String,
    /// Raw HTTP body for callers that need the original Dataverse payload.
    pub raw_body: Option<String>,
}

/// Supported Dataverse operations for a batch request.
#[derive(Debug, Clone)]
pub enum OrganizationRequest {
    /// Create a new Dataverse row.
    Create(CreateRequest),
    /// Update an existing Dataverse row.
    Update(UpdateRequest),
    /// Delete an existing Dataverse row.
    Delete(DeleteRequest),
}

/// Successful payload for a single `OrganizationRequest`.
#[derive(Debug, Clone)]
pub enum OrganizationResponse {
    /// Create response containing the created row id when Dataverse returns it.
    Create(CreateResponse),
    /// Update response placeholder for successful updates.
    Update(UpdateResponse),
    /// Delete response placeholder for successful deletes.
    Delete(DeleteResponse),
}

/// Create operation inside a batch request.
#[derive(Debug, Clone)]
pub struct CreateRequest {
    /// Entity payload to create.
    pub target: Entity,
    /// Optional Dataverse headers that affect plugin/business logic execution.
    pub parameters: RequestParameters,
}

/// Success payload for a batch create request.
#[derive(Debug, Clone)]
pub struct CreateResponse {
    /// Created row id extracted from Dataverse response headers when available.
    pub id: Option<Uuid>,
}

/// Update operation inside a batch request.
#[derive(Debug, Clone)]
pub struct UpdateRequest {
    /// Entity payload to update. The entity id must already be populated.
    pub target: Entity,
    /// Optional Dataverse headers that affect plugin/business logic execution.
    pub parameters: RequestParameters,
}

/// Success payload for a batch update request.
#[derive(Debug, Clone, Default)]
pub struct UpdateResponse;

/// Delete operation inside a batch request.
#[derive(Debug, Clone)]
pub struct DeleteRequest {
    /// Entity reference to delete.
    pub target: EntityReference,
    /// Optional Dataverse headers that affect plugin/business logic execution.
    pub parameters: RequestParameters,
}

/// Success payload for a batch delete request.
#[derive(Debug, Clone, Default)]
pub struct DeleteResponse;

#[derive(Debug, Clone)]
pub(crate) struct PreparedBatchRequest {
    pub(crate) method: &'static str,
    pub(crate) path: String,
    pub(crate) body: Option<String>,
    pub(crate) parameters: RequestParameters,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedBatchItem {
    pub(crate) prepared_request: PreparedBatchRequest,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedBatchPart {
    pub(crate) status_code: u16,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) body: Option<String>,
}

impl CreateRequest {
    /// Create a batch create request with default request parameters.
    pub fn new(target: Entity) -> Self {
        Self {
            target,
            parameters: RequestParameters::default(),
        }
    }
}

impl UpdateRequest {
    /// Create a batch update request with default request parameters.
    pub fn new(target: Entity) -> Self {
        Self {
            target,
            parameters: RequestParameters::default(),
        }
    }
}

impl DeleteRequest {
    /// Create a batch delete request with default request parameters.
    pub fn new(target: EntityReference) -> Self {
        Self {
            target,
            parameters: RequestParameters::default(),
        }
    }
}

impl OrganizationRequest {
    pub(crate) fn success_response(&self, headers: &HashMap<String, String>) -> OrganizationResponse {
        match self {
            OrganizationRequest::Create(_) => OrganizationResponse::Create(CreateResponse {
                id: entity_id_from_headers(headers),
            }),
            OrganizationRequest::Update(_) => OrganizationResponse::Update(UpdateResponse),
            OrganizationRequest::Delete(_) => OrganizationResponse::Delete(DeleteResponse),
        }
    }
}

pub(crate) fn entity_to_write_body(
    entity: &Entity,
    entity_set_name_by_logical_name: &HashMap<String, String>,
) -> Result<String, String> {
    let mut body = Map::new();

    for (attribute, value) in &entity.attributes {
        match value {
            DataverseValue::EntityReference(reference) => {
                let entity_set_name = entity_set_name_by_logical_name
                    .get(&reference.logical_name.to_ascii_lowercase())
                    .ok_or_else(|| {
                        format!(
                            "Entity set metadata not found for referenced entity '{}'",
                            reference.logical_name
                        )
                    })?;

                body.insert(
                    format!("{attribute}@odata.bind"),
                    JsonValue::String(format!(
                        "{entity_set_name}({})",
                        reference.id.as_hyphenated()
                    )),
                );
            }
            other => {
                body.insert(attribute.clone(), value_to_json(other)?);
            }
        }
    }

    serde_json::to_string(&body).map_err(|e| format!("Failed to serialize request body: {e}"))
}

pub(crate) fn parse_batch_response_parts(
    content_type: Option<&str>,
    response_text: &str,
) -> Result<Vec<ParsedBatchPart>, String> {
    let boundary = extract_boundary(
        content_type.ok_or_else(|| "Batch response missing Content-Type header".to_string())?,
    )?;
    parse_multipart_parts(response_text, &boundary)
}

fn value_to_json(value: &DataverseValue) -> Result<JsonValue, String> {
    match value {
        DataverseValue::Int(value) => Ok(JsonValue::Number(Number::from(*value))),
        DataverseValue::Float(value) => Number::from_f64(*value)
            .map(JsonValue::Number)
            .ok_or_else(|| format!("Cannot serialize non-finite float value: {value}")),
        DataverseValue::Decimal(value) => json_number_from_string(&value.to_string()),
        DataverseValue::String(value) => Ok(JsonValue::String(value.clone())),
        DataverseValue::Boolean(value) => Ok(JsonValue::Bool(*value)),
        DataverseValue::DateTime(value) => Ok(JsonValue::String(value.to_rfc3339())),
        DataverseValue::Guid(value) => Ok(JsonValue::String(value.as_hyphenated().to_string())),
        DataverseValue::Money(value) => json_number_from_string(&value.value.to_string()),
        DataverseValue::OptionSetValue(value) => Ok(JsonValue::Number(Number::from(value.value))),
        DataverseValue::OptionSetValueCollection(OptionSetValueCollection { values }) => Ok(
            JsonValue::String(
                values
                    .iter()
                    .map(i32::to_string)
                    .collect::<Vec<String>>()
                    .join(","),
            ),
        ),
        DataverseValue::Null => Ok(JsonValue::Null),
        DataverseValue::EntityReference(_) => unreachable!("entity references are handled separately"),
    }
}

fn json_number_from_string(value: &str) -> Result<JsonValue, String> {
    serde_json::from_str::<JsonValue>(value)
        .map_err(|e| format!("Failed to serialize numeric value '{value}': {e}"))
}

fn extract_boundary(content_type: &str) -> Result<String, String> {
    content_type
        .split(';')
        .map(str::trim)
        .find_map(|segment| segment.strip_prefix("boundary="))
        .map(|value| value.trim_matches('"').to_string())
        .ok_or_else(|| format!("Batch response missing boundary in Content-Type: {content_type}"))
}

fn parse_multipart_parts(payload: &str, boundary: &str) -> Result<Vec<ParsedBatchPart>, String> {
    let marker = format!("--{boundary}");
    let terminator = format!("--{boundary}--");
    let mut parts = Vec::new();

    for section in payload.split(&marker).skip(1) {
        let trimmed = section.trim();
        if trimmed.is_empty() || trimmed == "--" || trimmed == terminator {
            continue;
        }

        if trimmed.starts_with("--") {
            continue;
        }

        let normalized = trimmed.trim_matches('\r').trim_matches('\n');
        let Some((part_headers, part_body)) = normalized.split_once(HEADER_SEPARATOR) else {
            continue;
        };

        let headers = parse_headers(part_headers);
        let content_type = headers
            .get(&CONTENT_TYPE.as_str().to_ascii_lowercase())
            .cloned()
            .unwrap_or_default();

        if content_type.starts_with("multipart/mixed") {
            let nested_parts = parse_multipart_parts(part_body, &extract_boundary(&content_type)?)?;
            parts.extend(nested_parts);
            continue;
        }

        if !content_type.starts_with("application/http") {
            continue;
        }

        parts.push(parse_application_http_part(part_body)?);
    }

    Ok(parts)
}

fn parse_application_http_part(content: &str) -> Result<ParsedBatchPart, String> {
    let normalized = content.trim_matches('\r').trim_matches('\n');
    let (raw_headers, body) = normalized
        .split_once(HEADER_SEPARATOR)
        .map(|(headers, body)| (headers, Some(body.to_string())))
        .unwrap_or((normalized, None));

    let mut lines = raw_headers.lines();
    let status_line = lines
        .next()
        .ok_or_else(|| "Batch response item missing HTTP status line".to_string())?;

    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("Invalid batch response status line: {status_line}"))?
        .parse::<u16>()
        .map_err(|e| format!("Invalid batch response status code: {e}"))?;

    let headers = parse_headers(&lines.collect::<Vec<&str>>().join("\r\n"));

    Ok(ParsedBatchPart {
        status_code,
        headers,
        body: body.and_then(|body| {
            let trimmed = body.trim_matches('\r').trim_matches('\n').trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }),
    })
}

fn parse_headers(raw_headers: &str) -> HashMap<String, String> {
    raw_headers
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
        })
        .collect()
}

fn entity_id_from_headers(headers: &HashMap<String, String>) -> Option<Uuid> {
    ["odata-entityid", "location"]
        .into_iter()
        .filter_map(|name| headers.get(name))
        .find_map(|value| parse_uuid_from_uri(value))
}

fn parse_uuid_from_uri(value: &str) -> Option<Uuid> {
    let start = value.rfind('(')? + 1;
    let end = value.rfind(')')?;
    Uuid::parse_str(value[start..end].trim_matches('{').trim_matches('}')).ok()
}

pub(crate) fn parse_fault(part: &ParsedBatchPart) -> OrganizationServiceFault {
    let (code, message) = part
        .body
        .as_deref()
        .and_then(parse_fault_json)
        .unwrap_or((None, format!("Dataverse batch item failed with HTTP {}", part.status_code)));

    OrganizationServiceFault {
        status_code: part.status_code,
        code,
        message,
        raw_body: part.body.clone(),
    }
}

fn parse_fault_json(body: &str) -> Option<(Option<String>, String)> {
    let json: JsonValue = serde_json::from_str(body).ok()?;
    let error = json.get("error")?;
    let message = error.get("message")?.as_str()?.to_string();
    let code = error
        .get("code")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string());
    Some((code, message))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rust_decimal::Decimal;
    use uuid::Uuid;

    use super::{
        CreateRequest, OrganizationRequest, entity_to_write_body, parse_batch_response_parts,
        parse_fault,
    };
    use crate::dataverse::entity::{Entity, EntityReference, Money, Value};

    #[test]
    fn parses_flat_batch_response_parts() {
        let payload = concat!(
            "--batchresponse_123\r\n",
            "Content-Type: application/http\r\n",
            "Content-Transfer-Encoding: binary\r\n",
            "\r\n",
            "HTTP/1.1 204 No Content\r\n",
            "OData-EntityId: https://example.crm.dynamics.com/api/data/v9.2/accounts(aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee)\r\n",
            "\r\n",
            "--batchresponse_123\r\n",
            "Content-Type: application/http\r\n",
            "Content-Transfer-Encoding: binary\r\n",
            "\r\n",
            "HTTP/1.1 400 Bad Request\r\n",
            "Content-Type: application/json\r\n",
            "\r\n",
            "{\"error\":{\"code\":\"0x1\",\"message\":\"Bad data\"}}\r\n",
            "--batchresponse_123--\r\n"
        );

        let parts = parse_batch_response_parts(
            Some("multipart/mixed; boundary=batchresponse_123"),
            payload,
        )
        .expect("should parse multipart response");

        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].status_code, 204);
        assert_eq!(parts[1].status_code, 400);

        let fault = parse_fault(&parts[1]);
        assert_eq!(fault.code.as_deref(), Some("0x1"));
        assert_eq!(fault.message, "Bad data");
    }

    #[test]
    fn serializes_entity_reference_as_odata_bind() {
        let mut entity = Entity::new(Uuid::new_v4(), "contact", None);
        entity.attributes.insert(
            "parentcustomerid".to_string(),
            Value::EntityReference(EntityReference {
                id: Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").expect("uuid"),
                logical_name: "account".to_string(),
                name: None,
            }),
        );

        let body = entity_to_write_body(
            &entity,
            &HashMap::from([("account".to_string(), "accounts".to_string())]),
        )
        .expect("should serialize");

        assert!(body.contains("\"parentcustomerid@odata.bind\":\"accounts(aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee)\""));
    }

    #[test]
    fn create_request_constructor_uses_default_parameters() {
        let request = CreateRequest::new(Entity::new(Uuid::new_v4(), "account", None));

        assert!(request.parameters.headers().is_empty());
    }

    #[test]
    fn create_success_response_reads_entity_id_header() {
        let request = OrganizationRequest::Create(CreateRequest::new(Entity::new(
            Uuid::new_v4(),
            "account",
            None,
        )));

        let response = request.success_response(&HashMap::from([(
            "odata-entityid".to_string(),
            "https://example.crm.dynamics.com/api/data/v9.2/accounts(aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee)"
                .to_string(),
        )]));

        match response {
            super::OrganizationResponse::Create(created) => {
                assert_eq!(
                    created.id,
                    Some(Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").expect("uuid"))
                );
            }
            _ => panic!("expected create response"),
        }
    }

    #[test]
    fn serializes_decimal_like_values_without_quotes() {
        let mut entity = Entity::new(Uuid::new_v4(), "invoice", None);
        entity.attributes.insert(
            "totalamount".to_string(),
            Value::Money(Money {
                value: Decimal::new(12345, 2),
            }),
        );

        let body = entity_to_write_body(&entity, &HashMap::new()).expect("should serialize");

        assert!(body.contains("\"totalamount\":123.45"));
    }
}
