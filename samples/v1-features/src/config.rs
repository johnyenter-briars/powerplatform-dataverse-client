use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Secrets {
    #[serde(default)]
    pub device_code_connection_string: String,
    #[serde(default)]
    pub client_credentials_connection_string: String,
}

pub const BUILTIN_SAMPLE_TABLES: &[&str] = &[
    "account",
    "contact",
    "email",
    "activitypointer",
    "incident",
    "lead",
    "opportunity",
    "systemuser",
    "task",
];

pub fn load_secrets() -> Result<Secrets, String> {
    let mut path = std::env::current_dir().map_err(|e| e.to_string())?;
    path.push("secrets.json");
    read_secrets(&path)
}

fn read_secrets(path: &PathBuf) -> Result<Secrets, String> {
    let contents =
        fs::read_to_string(path).map_err(|e| format!("Failed to read secrets.json: {e}"))?;
    serde_json::from_str(&contents).map_err(|e| format!("Invalid secrets.json: {e}"))
}
