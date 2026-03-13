use std::collections::HashMap;
use std::fs;
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::credentials::{
    fetch_client_credentials_token_with_expiry, fetch_device_code_token_from_parts,
    parse_device_code_connection_string,
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

#[derive(Debug, Serialize, Deserialize)]
struct TokenCacheFile {
    access_token: String,
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
        /// Optional redirect URI from the original connection string.
        redirect_uri: Option<String>,
        /// Optional token cache path from the original connection string.
        token_cache_store_path: Option<String>,
        /// Optional login prompt from the original connection string.
        login_prompt: Option<String>,
        /// Optional username from the original connection string.
        username: Option<String>,
        /// Optional password from the original connection string.
        password: Option<String>,
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

fn parse_jwt_expiry(access_token: &str) -> Option<u64> {
    let payload = access_token.split('.').nth(1)?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    json.get("exp").and_then(|value| value.as_u64())
}

fn parse_connection_string_values(
    connection_string: &str,
) -> Result<HashMap<String, String>, String> {
    let mut values = HashMap::new();

    for segment in connection_string.split(';') {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            return Err(format!("Invalid connection string segment: {trimmed}"));
        };

        values.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
    }

    Ok(values)
}

fn parse_connection_string_auth_config(connection_string: &str) -> Result<AuthConfig, String> {
    let values = parse_connection_string_values(connection_string)?;

    let url = values
        .get("url")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or("Connection string missing Url".to_string())?;

    let client_id = values
        .get("clientid")
        .or_else(|| values.get("appid"))
        .cloned()
        .filter(|value| !value.trim().is_empty());
    let client_secret = values
        .get("clientsecret")
        .cloned()
        .filter(|value| !value.trim().is_empty());
    let tenant_id = values
        .get("tenantid")
        .cloned()
        .filter(|value| !value.trim().is_empty());
    let token_cache_store_path = values
        .get("tokencachestorepath")
        .cloned()
        .filter(|value| !value.trim().is_empty());

    if let (Some(client_id), Some(client_secret), Some(tenant_id)) =
        (client_id.clone(), client_secret, tenant_id.clone())
    {
        return Ok(AuthConfig::ClientCredentials {
            client_id,
            client_secret,
            tenant_id,
            scope: format!("{}/.default", url.trim_end_matches('/')),
            token_cache_store_path,
        });
    }

    let parsed = parse_device_code_connection_string(connection_string)?;

    Ok(AuthConfig::DeviceCode {
        client_id: parsed.client_id,
        dataverse_url: parsed.dataverse_url,
        tenant_id: parsed.tenant_id,
        redirect_uri: parsed.redirect_uri,
        token_cache_store_path: parsed.token_cache_store_path,
        login_prompt: parsed.login_prompt,
        username: parsed.username,
        password: parsed.password,
    })
}

async fn fetch_token_for_config(auth: &AuthConfig) -> Result<CachedToken, String> {
    match auth {
        AuthConfig::ClientCredentials {
            client_id,
            client_secret,
            tenant_id,
            scope,
            ..
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
        AuthConfig::DeviceCode {
            client_id,
            dataverse_url,
            tenant_id,
            ..
        } => {
            let access_token =
                fetch_device_code_token_from_parts(client_id, dataverse_url, tenant_id).await?;

            Ok(CachedToken {
                expires_at: parse_jwt_expiry(&access_token),
                access_token,
            })
        }
    }
}

/// Fetch a fresh access token from a Dataverse-style device-code connection string.
pub async fn fetch_token(connection_string: &str) -> Result<CachedToken, String> {
    let auth = parse_connection_string_auth_config(connection_string)?;
    if let Some(cached) = load_cached_token(&auth)? {
        if !cached.access_token.trim().is_empty() && !is_expiring_soon(cached.expires_at) {
            return Ok(cached);
        }
    }

    let token = fetch_token_for_config(&auth).await?;
    save_cached_token(&auth, &token)?;
    Ok(token)
}

/// Populate a token cache with a valid token for the given key.
pub async fn prime_token_cache<K: Eq + Hash + Clone>(
    auth: &AuthConfig,
    cache: &mut HashMap<K, CachedToken>,
    key: K,
) -> Result<(), String> {
    let token = fetch_token_for_config(auth).await?;
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

    let refreshed = fetch_token_for_config(auth).await?;
    let access_token = refreshed.access_token.clone();
    cache.insert(key.clone(), refreshed);
    Ok(access_token)
}

fn load_cached_token(
    auth: &AuthConfig,
) -> Result<Option<CachedToken>, String> {
    let path = resolve_token_cache_file_path(auth)?;
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    if contents.trim().is_empty() {
        return Ok(None);
    }

    let cache: TokenCacheFile = serde_json::from_str(&contents).map_err(|e| e.to_string())?;
    if cache.access_token.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(CachedToken {
        expires_at: parse_jwt_expiry(&cache.access_token),
        access_token: cache.access_token,
    }))
}

fn save_cached_token(auth: &AuthConfig, token: &CachedToken) -> Result<(), String> {
    let path = resolve_token_cache_file_path(auth)?;
    let parent = path
        .parent()
        .ok_or("Token cache path did not have a parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;

    let cache = TokenCacheFile {
        access_token: token.access_token.clone(),
    };
    let json = serde_json::to_string_pretty(&cache).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn resolve_token_cache_file_path(
    auth: &AuthConfig,
) -> Result<PathBuf, String> {
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
