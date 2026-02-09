pub mod auth;
pub mod dataverse;

#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Information,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Information
    }
}
