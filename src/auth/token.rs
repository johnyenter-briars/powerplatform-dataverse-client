use std::collections::HashMap;
use std::hash::Hash;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::credentials::{
    fetch_client_credentials_token_with_expiry, refresh_authorization_token,
};

const REFRESH_SKEW_SECS: u64 = 300;

/// Cached access token and optional expiry.
#[derive(Clone, Debug)]
pub struct CachedToken {
    /// OAuth access token.
    pub access_token: String,
    /// Expiration time as seconds since epoch.
    pub expires_at: Option<u64>,
}

/// Authentication configuration for acquiring Dataverse access tokens.
#[derive(Clone, Debug)]
pub enum AuthConfig {
    /// Client credentials (app-only) flow configuration.
    ClientCredentials {
        /// Azure AD client ID.
        client_id: String,
        /// Azure AD client secret.
        client_secret: String,
        /// Azure AD tenant ID.
        tenant_id: String,
        /// OAuth scope string.
        scope: String,
    },
    /// Authorization code flow configuration.
    AuthorizationCode {
        /// Azure AD client ID.
        client_id: String,
        /// Azure AD client secret.
        client_secret: String,
        /// Azure AD tenant ID.
        tenant_id: String,
        /// OAuth scope string.
        scope: String,
        /// Current access token.
        access_token: String,
        /// Refresh token for renewing the access token.
        refresh_token: String,
        /// Expiration time as seconds since epoch.
        expires_at: Option<u64>,
    },
}

/// Current timestamp in seconds since epoch.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Returns true if the token is missing or nearing expiry.
pub fn is_expiring_soon(expires_at: Option<u64>) -> bool {
    let Some(exp) = expires_at else {
        return true;
    };
    now_secs() + REFRESH_SKEW_SECS >= exp
}

/// Parse an expiry timestamp from a string.
pub fn parse_expires_at(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

/// Fetch a fresh access token for the provided auth configuration.
pub async fn fetch_token(auth: &AuthConfig) -> Result<CachedToken, String> {
    match auth {
        AuthConfig::ClientCredentials {
            client_id,
            client_secret,
            tenant_id,
            scope,
        } => {
            let token = fetch_client_credentials_token_with_expiry(
                client_id,
                client_secret,
                tenant_id,
                scope,
            )
            .await?;

            Ok(CachedToken {
                access_token: token.access_token,
                expires_at: Some(token.expires_at),
            })
        }
        AuthConfig::AuthorizationCode {
            client_id,
            client_secret,
            tenant_id,
            scope,
            refresh_token,
            ..
        } => {
            todo!("#11");

            if client_id.trim().is_empty()
                || client_secret.trim().is_empty()
                || tenant_id.trim().is_empty()
                || scope.trim().is_empty()
            {
                return Err(
                    "Authorization code connection cannot refresh without client credentials."
                        .to_string(),
                );
            }

            let token = refresh_authorization_token(
                client_id,
                client_secret,
                tenant_id,
                scope,
                refresh_token,
            )
            .await?;

            Ok(CachedToken {
                access_token: token.access_token,
                expires_at: Some(token.expires_at),
            })
        }
    }
}

/// Populate a token cache with a valid token for the given key.
pub async fn prime_token_cache<K: Eq + Hash + Clone>(
    auth: &AuthConfig,
    cache: &mut HashMap<K, CachedToken>,
    key: K,
) -> Result<(), String> {
    let token = match auth {
        AuthConfig::ClientCredentials { .. } => fetch_token(auth).await?,
        AuthConfig::AuthorizationCode {
            access_token,
            expires_at,
            ..
        } => {
            todo!("#11");
            let cached = CachedToken {
                access_token: access_token.clone(),
                expires_at: *expires_at,
            };

            if access_token.trim().is_empty() || is_expiring_soon(*expires_at) {
                fetch_token(auth).await?
            } else {
                cached
            }
        }
    };

    cache.insert(key, token);
    Ok(())
}

/// Return a valid access token from cache or by fetching a new one.
pub async fn get_access_token<K: Eq + Hash + Clone>(
    auth: &AuthConfig,
    cache: &mut HashMap<K, CachedToken>,
    key: &K,
) -> Result<String, String> {
    if let Some(cached) = cache.get(key) {
        if !cached.access_token.trim().is_empty() && !is_expiring_soon(cached.expires_at) {
            return Ok(cached.access_token.clone());
        }
    }

    let refreshed = fetch_token(auth).await?;
    let access_token = refreshed.access_token.clone();
    cache.insert(key.clone(), refreshed);
    Ok(access_token)
}
