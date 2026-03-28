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
    // Step-specific bypass ids are intentionally omitted for now because they need a more stable
    // public shape than a raw string list. The current API only exposes the simple boolean-style
    // switches that map cleanly to well-known headers.
    // pub bypass_business_logic_execution_step_ids: Option<Vec<String>>,
}

impl RequestParameters {
    /// Return the Dataverse request headers represented by these parameters.
    pub fn headers(&self) -> Vec<(&'static str, &'static str)> {
        let mut headers = Vec::new();

        if let Some(value) = self.bypass_business_logic_execution_value() {
            headers.push(("MSCRM.BypassBusinessLogicExecution", value));
        }

        if self.bypass_custom_plugin_execution {
            headers.push(("MSCRM.BypassCustomPluginExecution", "true"));
        }

        if self.suppress_callback_registration_expander_job {
            headers.push(("MSCRM.SuppressCallbackRegistrationExpanderJob", "true"));
        }

        headers
    }

    /// Apply the configured Dataverse request parameters to an outgoing request.
    pub fn apply(&self, mut request: RequestBuilder) -> RequestBuilder {
        for (header, value) in self.headers() {
            request = request.header(header, value);
        }

        // Step-id bypass headers are not emitted yet for the same reason documented on the struct:
        // the crate does not currently expose a stable typed API for managing those ids.
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
