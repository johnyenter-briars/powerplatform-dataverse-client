/// Public authentication configuration for acquiring Dataverse access tokens.
#[derive(Clone, Debug)]
pub enum AuthConfig {
    /// Azure AD client credentials (app-only) flow configuration.
    ClientCredentials {
        /// Azure AD client ID.
        client_id: String,
        /// Azure AD client secret.
        client_secret: String,
        /// Azure AD tenant ID.
        tenant_id: String,
        /// Dataverse environment URL.
        dataverse_url: String,
        /// Optional token cache path from the original connection string.
        token_cache_store_path: Option<String>,
    },
    /// Device code flow configuration.
    DeviceCode {
        /// Azure AD client ID.
        client_id: String,
        /// Dataverse environment URL.
        dataverse_url: String,
        /// Azure AD tenant ID.
        tenant_id: String,
        /// Optional token cache path from the original connection string.
        token_cache_store_path: Option<String>,
    },
}

impl AuthConfig {
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
