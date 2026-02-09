/// Authentication helpers for Dataverse and Microsoft identity flows.
pub mod auth;
/// Dataverse-specific types and service client helpers.
pub mod dataverse;

/// Logging verbosity for SDK operations.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Emit verbose debug output.
    Debug,
    /// Emit standard informational output.
    Information,
}

impl Default for LogLevel {
    /// Defaults to `Information` logging.
    fn default() -> Self {
        LogLevel::Information
    }
}
