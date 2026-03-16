use crate::auth::config::AuthConfig;
use crate::auth::token::{
    fetch_token_for_config_with_progress, is_expiring_soon, load_cached_token,
    resolve_token_cache_file_path, save_cached_token,
};

/// Progress updates emitted during OAuth device code authentication.
#[derive(Clone, Debug)]
pub enum DeviceCodeFlowEvent {
    Code {
        verification_uri: String,
        verification_uri_complete: Option<String>,
        user_code: String,
        message: Option<String>,
    },
    Waiting,
    Success,
}

/// Ensure a valid cached device-code token exists while reporting progress to the caller.
pub async fn ensure_device_code_token_with_progress<F>(
    auth: &AuthConfig,
    progress: F,
) -> Result<(), String>
where
    F: Fn(DeviceCodeFlowEvent) + Send + Sync,
{
    let AuthConfig::DeviceCode { .. } = auth else {
        return Err("Device code progress is only supported for device code auth".to_string());
    };

    let token_cache_path = resolve_token_cache_file_path(auth)?;
    if let Some(cached) = load_cached_token(&token_cache_path)?
        && !cached.access_token.trim().is_empty() && !is_expiring_soon(cached.expires_at) {
            return Ok(());
        }

    let token = fetch_token_for_config_with_progress(auth, Some(&progress)).await?;
    save_cached_token(&token_cache_path, &token)?;
    Ok(())
}
