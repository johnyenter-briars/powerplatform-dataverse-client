use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::config::AuthConfig;
use crate::auth::credentials::{
    fetch_client_credentials_token_with_expiry, fetch_device_code_token_exchange_from_parts,
};

const REFRESH_SKEW_SECS: u64 = 300;

/// Cached access token and optional expiry.
#[derive(Clone, Debug)]
pub(crate) struct CachedToken {
    /// OAuth access token.
    pub access_token: String,
    /// OAuth refresh token, when the auth flow provides one.
    pub refresh_token: Option<String>,
    /// Expiration time as seconds since epoch.
    pub expires_at: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenCacheFile {
    access_token: String,
    refresh_token: Option<String>,
}

/// Current timestamp in seconds since epoch.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Returns true if the token is missing or nearing expiry.
pub(crate) fn is_expiring_soon(expires_at: Option<u64>) -> bool {
    let Some(exp) = expires_at else {
        return true;
    };
    now_secs() + REFRESH_SKEW_SECS >= exp
}

fn parse_jwt_expiry(access_token: &str) -> Option<u64> {
    let payload = access_token.split('.').nth(1)?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    json.get("exp").and_then(|value| value.as_u64())
}

pub(crate) async fn fetch_token_for_config(auth: &AuthConfig) -> Result<CachedToken, String> {
    match auth {
        AuthConfig::ClientCredentials {
            client_id,
            client_secret,
            tenant_id,
            ..
        } => {
            let scope = auth
                .scope()
                .ok_or("Client credentials auth config missing scope".to_string())?;
            let token = fetch_client_credentials_token_with_expiry(
                client_id,
                client_secret,
                tenant_id,
                &scope,
            )
            .await?;

            Ok(CachedToken {
                access_token: token.access_token,
                refresh_token: None,
                expires_at: Some(token.expires_at),
            })
        }
        AuthConfig::DeviceCode {
            client_id,
            dataverse_url,
            tenant_id,
            ..
        } => {
            let token =
                fetch_device_code_token_exchange_from_parts(client_id, dataverse_url, tenant_id)
                    .await?;

            Ok(CachedToken {
                expires_at: Some(token.expires_at),
                refresh_token: Some(token.refresh_token),
                access_token: token.access_token,
            })
        }
    }
}

pub(crate) fn load_cached_token(path: &Path) -> Result<Option<CachedToken>, String> {
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if contents.trim().is_empty() {
        return Ok(None);
    }

    let cache: TokenCacheFile = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
    if cache.access_token.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(CachedToken {
        expires_at: parse_jwt_expiry(&cache.access_token),
        refresh_token: cache.refresh_token,
        access_token: cache.access_token,
    }))
}

pub(crate) fn save_cached_token(path: &Path, token: &CachedToken) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Token cache path did not have a parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let cache = TokenCacheFile {
        access_token: token.access_token.clone(),
        refresh_token: token.refresh_token.clone(),
    };
    let json = serde_json::to_string_pretty(&cache).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub(crate) fn resolve_token_cache_file_path(auth: &AuthConfig) -> Result<PathBuf, String> {
    if let Some(configured) = configured_token_cache_path(auth) {
        return normalize_token_cache_path(configured);
    }

    let base = dirs::data_local_dir().ok_or("Unable to resolve local app data directory")?;
    let cache_dir = base
        .join("powerplatform-dataverse-client")
        // TODO: Make this more deterministic to not cause lots of noise.
        .join(Uuid::new_v4().to_string());
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    Ok(cache_dir.join("token_cache.txt"))
}

fn configured_token_cache_path(auth: &AuthConfig) -> Option<&str> {
    match auth {
        AuthConfig::ClientCredentials {
            token_cache_store_path,
            ..
        } => token_cache_store_path.as_deref(),
        AuthConfig::DeviceCode {
            token_cache_store_path,
            ..
        } => token_cache_store_path.as_deref(),
    }
}

fn normalize_token_cache_path(value: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(value);

    if path.exists() {
        if path.is_dir() {
            fs::create_dir_all(&path).map_err(|e| e.to_string())?;
            return Ok(path.join("token_cache.txt"));
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        return Ok(path);
    }

    if looks_like_file_path(&path) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        return Ok(path);
    }

    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path.join("token_cache.txt"))
}

fn looks_like_file_path(path: &Path) -> bool {
    path.extension().is_some()
}
