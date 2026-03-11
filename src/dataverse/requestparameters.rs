use reqwest::RequestBuilder;

#[derive(Debug, Clone, Default)]
pub struct RequestParameters {
    pub bypass_business_logic_execution_custom_sync: bool,
    pub bypass_business_logic_execution_custom_async: bool,
    pub bypass_custom_plugin_execution: bool,
    pub suppress_callback_registration_expander_job: bool,
    // TODO:
    // pub bypass_business_logic_execution_step_ids: Option<Vec<String>>,
}

impl RequestParameters {
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
