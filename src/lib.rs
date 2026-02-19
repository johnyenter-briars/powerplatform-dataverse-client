use log::LevelFilter;

/// Authentication helpers for Dataverse and Microsoft identity flows.
pub mod auth;
/// Dataverse-specific types and service client helpers.
pub mod dataverse;

/// Logging verbosity for SDK operations.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Emit error output only.
    Error,
    /// Emit warning and error output.
    Warn,
    /// Emit standard informational output.
    Information,
    /// Emit verbose debug output.
    Debug,
    /// Emit verbose trace output.
    Trace,
}

impl LogLevel {
    /// Convert the SDK log level to a `log` crate filter.
    pub fn as_filter(self) -> LevelFilter {
        match self {
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Information => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }

    /// Whether debug-style messages should be emitted by the SDK.
    pub fn includes_debug(self) -> bool {
        matches!(self, LogLevel::Debug | LogLevel::Trace)
    }
}

impl Default for LogLevel {
    /// Defaults to `Error` logging.
    fn default() -> Self {
        LogLevel::Error
    }
}
