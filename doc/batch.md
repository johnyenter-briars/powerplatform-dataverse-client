# Batch Operations

The crate exposes an `ExecuteMultiple`-style batch model over the Dataverse Web API `$batch` endpoint.

Microsoft Learn background:

- [Execute batch operations using the Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/execute-batch-operations-using-web-api)

## Public API

### Settings and envelopes

- `ExecuteMultipleSettings`
- `ExecuteMultipleRequest`
- `ExecuteMultipleResponse`
- `ExecuteMultipleResponseItem`
- `OrganizationServiceFault`

### Request and response unions

- `OrganizationRequest`
- `OrganizationResponse`

### Typed request and response models

- `CreateRequest`
- `CreateResponse`
- `UpdateRequest`
- `UpdateResponse`
- `DeleteRequest`
- `DeleteResponse`

### Constructors

- `CreateRequest::new(target: Entity) -> CreateRequest`
- `UpdateRequest::new(target: Entity) -> UpdateRequest`
- `DeleteRequest::new(target: EntityReference) -> DeleteRequest`

### Service client entry point

- `ServiceClient::execute_multiple(&self, request: &ExecuteMultipleRequest) -> Result<ExecuteMultipleResponse, String>`

## Notes

- Requests are executed in the order supplied.
- `continue_on_error` maps to Dataverse's `Prefer: odata.continue-on-error` behavior.
- `return_responses` controls whether successful items are surfaced in the returned `ExecuteMultipleResponse`.
- The current implementation targets create, update, and delete batch patterns.

## Sample

See [`samples/v1-features/src/scenarios/batch.rs`](../samples/v1-features/src/scenarios/batch.rs).
