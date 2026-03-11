use reqwest::RequestBuilder;

/// Optional Dataverse request parameters for create and update operations.
#[derive(Debug, Clone, Default)]
pub struct RequestParameters {
    /// Send `MSCRM.BypassBusinessLogicExecution=CustomSync`.
    pub bypass_business_logic_execution_custom_sync: bool,
    /// Send `MSCRM.BypassBusinessLogicExecution=CustomAsync`.
    pub bypass_business_logic_execution_custom_async: bool,
    /// Send `MSCRM.BypassCustomPluginExecution=true`.
    pub bypass_custom_plugin_execution: bool,
    /// Send `MSCRM.SuppressCallbackRegistrationExpanderJob=true`.
    pub suppress_callback_registration_expander_job: bool,
    // Not implemented yet:
    // `MSCRM.BypassBusinessLogicExecutionStepIds` support for specific step IDs.
    // TODO:
    // pub bypass_business_logic_execution_step_ids: Option<Vec<String>>,
}

impl RequestParameters {
    /// Apply the configured Dataverse request parameters to an outgoing request.
    pub fn apply(&self, mut request: RequestBuilder) -> RequestBuilder {
        if let Some(value) = self.bypass_business_logic_execution_value() {
            request = request.header("MSCRM.BypassBusinessLogicExecution", value);
        }

        if self.bypass_custom_plugin_execution {
            request = request.header("MSCRM.BypassCustomPluginExecution", "true");
        }

        if self.suppress_callback_registration_expander_job {
            request = request.header("MSCRM.SuppressCallbackRegistrationExpanderJob", "true");
        }

        // TODO:
        // if let Some(step_ids) = &self.bypass_business_logic_execution_step_ids {
        //     request = request.header(
        //         "MSCRM.BypassBusinessLogicExecutionStepIds",
        //         step_ids.join(","),
        //     );
        // }

        request
    }

    /// Compose the `MSCRM.BypassBusinessLogicExecution` header value.
    fn bypass_business_logic_execution_value(&self) -> Option<&'static str> {
        match (
            self.bypass_business_logic_execution_custom_sync,
            self.bypass_business_logic_execution_custom_async,
        ) {
            (true, true) => Some("CustomSync,CustomAsync"),
            (true, false) => Some("CustomSync"),
            (false, true) => Some("CustomAsync"),
            (false, false) => None,
        }
    }
}
