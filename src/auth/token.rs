use std::collections::HashMap;
use std::hash::Hash;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::credentials::{
    fetch_client_credentials_token_with_expiry, refresh_authorization_token,
};

const REFRESH_SKEW_SECS: u64 = 300;

#[derive(Clone, Debug)]
pub struct CachedToken {
    pub access_token: String,
    pub expires_at: Option<u64>,
}

#[derive(Clone, Debug)]
pub enum AuthConfig {
    ClientCredentials {
        client_id: String,
        client_secret: String,
        tenant_id: String,
        scope: String,
    },
    AuthorizationCode {
        client_id: String,
        client_secret: String,
        tenant_id: String,
        scope: String,
        access_token: String,
        refresh_token: String,
        expires_at: Option<u64>,
    },
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn is_expiring_soon(expires_at: Option<u64>) -> bool {
    let Some(exp) = expires_at else {
        return true;
    };
    now_secs() + REFRESH_SKEW_SECS >= exp
}

pub fn parse_expires_at(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok()
}

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
