use reqwest::RequestBuilder;

#[derive(Debug, Clone, Default)]
pub struct RequestParameters {
    pub bypass_custom_plugin_execution: bool,
    pub suppress_callback_registration_expander_job: bool,
}

impl RequestParameters {
    pub fn apply(&self, mut request: RequestBuilder) -> RequestBuilder {
        if self.bypass_custom_plugin_execution {
            request = request.header("MSCRM.BypassCustomPluginExecution", "true");
        }

        if self.suppress_callback_registration_expander_job {
            request = request.header("MSCRM.SuppressCallbackRegistrationExpanderJob", "true");
        }

        request
    }
}
