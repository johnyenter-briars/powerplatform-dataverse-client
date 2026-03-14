# FetchXML

The client supports FetchXML retrieval, paging, and a count helper.

## Supported Methods

- `retrieve_multiple_fetchxml`
- `retrieve_multiple_fetchxml_count`

## Notes

- Paging is handled internally when the FetchXML query does not specify `top`.
- Aggregate queries are capped internally to a safe page size.
- Returned rows include an internal row number attribute for paging scenarios.

## Sample Scenario

See [`samples/v1-features/src/scenarios/fetchxml.rs`](../samples/v1-features/src/scenarios/fetchxml.rs).
