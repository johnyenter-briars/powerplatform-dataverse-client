# Logging

`powerplatform-dataverse-client` exposes a single public logging type:

- `LogLevel`

Microsoft Learn background:

- [Use the Microsoft Dataverse Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/overview)

`LogLevel` is crate-local infrastructure rather than a Dataverse feature. It controls how much request and paging information the client prints while working with the Dataverse Web API.

## Public API

### `LogLevel`

Variants:

- `Error`
- `Warn`
- `Information`
- `Debug`
- `Trace`

Methods:

- `LogLevel::as_filter(self) -> log::LevelFilter`
- `LogLevel::includes_debug(self) -> bool`

## Notes

- `Information` is the practical default when you want normal request visibility.
- `Debug` and `Trace` are mainly useful when diagnosing FetchXML paging, raw URLs, or auth-related request flow.
- `as_filter` is useful when wiring the crate into a broader Rust logging setup.
