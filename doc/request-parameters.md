# Request Parameters

`RequestParameters` maps supported Dataverse optional request headers onto create, update, and delete requests.

Microsoft Learn background:

- [Bypass custom business logic](https://learn.microsoft.com/power-apps/developer/data-platform/bypass-custom-business-logic)

## Public API

### `RequestParameters`

Fields:

- `bypass_business_logic_execution_custom_sync`
- `bypass_business_logic_execution_custom_async`
- `bypass_custom_plugin_execution`
- `suppress_callback_registration_expander_job`

Methods:

- `RequestParameters::headers(&self) -> Vec<(&'static str, &'static str)>`
- `RequestParameters::apply(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder`

## Header Mapping

| Field | Dataverse Header |
| --- | --- |
| `bypass_business_logic_execution_custom_sync` | `MSCRM.BypassBusinessLogicExecution=CustomSync` |
| `bypass_business_logic_execution_custom_async` | `MSCRM.BypassBusinessLogicExecution=CustomAsync` |
| `bypass_custom_plugin_execution` | `MSCRM.BypassCustomPluginExecution=true` |
| `suppress_callback_registration_expander_job` | `MSCRM.SuppressCallbackRegistrationExpanderJob=true` |

## Notes

- The current API covers the simple boolean-style headers that map cleanly to stable public fields.
- `MSCRM.BypassBusinessLogicExecutionStepIds` is not exposed yet.
- The `*_with_options` methods on `ServiceClient` are the intended place to use `RequestParameters`.

## Sample

See [`samples/v1-features/src/scenarios/request_parameters.rs`](../samples/v1-features/src/scenarios/request_parameters.rs).
