use serde::{Deserialize, Serialize};

use crate::auth::connectionstring::parse_connection_string_auth_config;

/// Public authentication configuration for acquiring Dataverse access tokens.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "method")]
pub enum AuthConfig {
    /// Azure AD client credentials (app-only) flow configuration.
    #[serde(rename = "ClientCredentials", alias = "ClientSecret")]
    ClientCredentials {
        /// Azure AD client ID.
        #[serde(rename = "clientId")]
        client_id: String,
        /// Azure AD client secret.
        #[serde(rename = "clientSecret")]
        client_secret: String,
        /// Azure AD tenant ID.
        #[serde(rename = "tenantId")]
        tenant_id: String,
        /// Dataverse environment URL.
        #[serde(rename = "dataverseUrl")]
        dataverse_url: String,
        /// Optional token cache path from the original connection string.
        #[serde(default)]
        #[serde(rename = "tokenCacheStorePath")]
        token_cache_store_path: Option<String>,
    },
    /// Device code flow configuration.
    #[serde(rename = "DeviceCode", alias = "AuthorizationCode", alias = "OAuth")]
    DeviceCode {
        /// Azure AD client ID.
        #[serde(rename = "clientId")]
        client_id: String,
        /// Dataverse environment URL.
        #[serde(rename = "dataverseUrl")]
        dataverse_url: String,
        /// Azure AD tenant ID.
        #[serde(default)]
        #[serde(rename = "tenantId")]
        tenant_id: String,
        /// Optional token cache path from the original connection string.
        #[serde(default)]
        #[serde(rename = "tokenCacheStorePath")]
        token_cache_store_path: Option<String>,
    },
}

impl AuthConfig {
    /// Parse an `AuthConfig` from a Dataverse-style connection string.
    pub fn from_connection_string(connection_string: &str) -> Result<Self, String> {
        parse_connection_string_auth_config(connection_string)
    }

    pub(crate) fn dataverse_url(&self) -> &str {
        match self {
            AuthConfig::ClientCredentials { dataverse_url, .. } => dataverse_url.trim_end_matches('/'),
            AuthConfig::DeviceCode { dataverse_url, .. } => dataverse_url,
        }
    }

    pub(crate) fn scope(&self) -> Option<String> {
        match self {
            AuthConfig::ClientCredentials { dataverse_url, .. } => {
                Some(format!("{}/.default", dataverse_url.trim_end_matches('/')))
            }
            AuthConfig::DeviceCode { .. } => None,
        }
    }
}
