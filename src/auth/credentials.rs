use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::Client;
use serde_json::Value;
use tokio::time::{Duration, sleep};

/// Result of exchanging an authorization code or refresh token.
pub struct TokenExchange {
    /// OAuth access token.
    pub access_token: String,
    /// OAuth refresh token.
    pub refresh_token: String,
    /// Expiration time as seconds since epoch.
    pub expires_at: u64,
}

/// Access token returned from the client credentials flow.
pub struct ClientCredentialsToken {
    /// OAuth access token.
    pub access_token: String,
    /// Expiration time as seconds since epoch.
    pub expires_at: u64,
}

struct DeviceCodeStart {
    device_code: String,
    expires_in: u64,
    interval: u64,
}

/// Fetch a client-credentials token along with its expiry timestamp.
pub(crate) async fn fetch_client_credentials_token_with_expiry(
    client_id: &str,
    client_secret: &str,
    tenant_id: &str,
    scope: &str,
) -> Result<ClientCredentialsToken, String> {
    let client = Client::new();
    let token_url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        tenant_id
    );

    let mut params = HashMap::new();
    params.insert("client_id", client_id);
    params.insert("client_secret", client_secret);
    params.insert("scope", scope);
    params.insert("grant_type", "client_credentials");

    let resp = client
        .post(&token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(body);
    }

    let json: Value = resp.json().await.map_err(|e| e.to_string())?;

    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("No access_token in response")?;
    let expires_in = json
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .ok_or("No expires_in in response")?;

    if access_token.trim().is_empty() {
        return Err("Access token was empty".to_string());
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    Ok(ClientCredentialsToken {
        access_token: access_token.to_string(),
        expires_at: now + expires_in,
    })
}

pub(crate) async fn fetch_device_code_token_exchange_from_parts(
    client_id: &str,
    dataverse_url: &str,
    tenant_id: &str,
) -> Result<TokenExchange, String> {
    let scope = build_dataverse_device_code_scope(dataverse_url);
    let client = Client::new();
    let start = start_device_code_flow(&client, tenant_id, client_id, &scope).await?;

    poll_device_code_token(
        &client,
        tenant_id,
        client_id,
        &scope,
        &start.device_code,
        start.interval,
        start.expires_in,
    )
    .await
}

pub(crate) async fn refresh_device_code_token(
    client_id: &str,
    tenant_id: &str,
    scope: &str,
    refresh_token: &str,
) -> Result<TokenExchange, String> {
    refresh_token_exchange(client_id, None, tenant_id, scope, refresh_token).await
}

fn build_dataverse_device_code_scope(dataverse_url: &str) -> String {
    format!(
        "{}/user_impersonation offline_access openid profile",
        dataverse_url
    )
}

async fn start_device_code_flow(
    client: &Client,
    tenant_id: &str,
    client_id: &str,
    scope: &str,
) -> Result<DeviceCodeStart, String> {
    let device_code_url =
        format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/devicecode");

    let mut params = HashMap::new();
    params.insert("client_id", client_id);
    params.insert("scope", scope);

    let resp = client
        .post(&device_code_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(body);
    }

    let json: Value = resp.json().await.map_err(|e| e.to_string())?;

    let device_code = json
        .get("device_code")
        .and_then(|v| v.as_str())
        .ok_or("No device_code in response")?
        .to_string();
    let expires_in = json
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .ok_or("No expires_in in response")?;
    let interval = json.get("interval").and_then(|v| v.as_u64()).unwrap_or(5);
    let user_code = json
        .get("user_code")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let verification_uri = json
        .get("verification_uri")
        .and_then(|v| v.as_str())
        .unwrap_or("https://microsoft.com/devicelogin");
    let verification_uri_complete = json
        .get("verification_uri_complete")
        .and_then(|v| v.as_str());

    if let Some(complete_uri) = verification_uri_complete {
        println!("Open this URL in your browser: {complete_uri}");
    } else {
        println!("Open this URL in your browser: {verification_uri}");
    }

    if !user_code.is_empty() {
        println!("Enter this code if prompted: {user_code}");
    }

    if let Some(message) = json.get("message").and_then(|v| v.as_str()) {
        println!("{message}");
    }

    Ok(DeviceCodeStart {
        device_code,
        expires_in,
        interval,
    })
}

async fn poll_device_code_token(
    client: &Client,
    tenant_id: &str,
    client_id: &str,
    scope: &str,
    device_code: &str,
    interval: u64,
    expires_in: u64,
) -> Result<TokenExchange, String> {
    let token_url = format!("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token");
    let started_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let mut poll_interval = interval.max(1);

    loop {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();
        if now.saturating_sub(started_at) >= expires_in {
            return Err("Device code expired before authentication completed".to_string());
        }

        let mut params = HashMap::new();
        params.insert("grant_type", "urn:ietf:params:oauth:grant-type:device_code");
        params.insert("client_id", client_id);
        params.insert("device_code", device_code);
        params.insert("scope", scope);

        let resp = client
            .post(&token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if resp.status().is_success() {
            let json: Value = resp.json().await.map_err(|e| e.to_string())?;
            let access_token = json
                .get("access_token")
                .and_then(|v| v.as_str())
                .ok_or("No access_token in response")?
                .to_string();
            let refresh_token = json
                .get("refresh_token")
                .and_then(|v| v.as_str())
                .ok_or("No refresh_token in response")?
                .to_string();
            let expires_in = json
                .get("expires_in")
                .and_then(|v| v.as_u64())
                .ok_or("No expires_in in response")?;

            if access_token.trim().is_empty() {
                return Err("Access token was empty".to_string());
            }

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs();

            return Ok(TokenExchange {
                access_token,
                refresh_token,
                expires_at: now + expires_in,
            });
        }

        let json: Value = resp.json().await.map_err(|e| e.to_string())?;
        let error = json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        match error {
            "authorization_pending" => {
                sleep(Duration::from_secs(poll_interval)).await;
            }
            "slow_down" => {
                poll_interval += 5;
                sleep(Duration::from_secs(poll_interval)).await;
            }
            "authorization_declined" => {
                return Err("Device code authentication was declined in the browser".to_string());
            }
            "expired_token" => {
                return Err("Device code expired before authentication completed".to_string());
            }
            "bad_verification_code" => {
                return Err("Device code was rejected by the identity provider".to_string());
            }
            _ => {
                return Err(json.to_string());
            }
        }
    }
}

async fn refresh_token_exchange(
    client_id: &str,
    client_secret: Option<&str>,
    tenant_id: &str,
    scope: &str,
    refresh_token: &str,
) -> Result<TokenExchange, String> {
    let client = Client::new();
    let token_url = format!(
        "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
        tenant_id
    );

    let mut params = HashMap::new();
    params.insert("client_id", client_id);
    params.insert("scope", scope);
    params.insert("grant_type", "refresh_token");
    params.insert("refresh_token", refresh_token);

    if let Some(client_secret) = client_secret.filter(|value| !value.trim().is_empty()) {
        params.insert("client_secret", client_secret);
    }

    let resp = client
        .post(&token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(body);
    }

    let json: Value = resp.json().await.map_err(|e| e.to_string())?;

    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or("No access_token in response")?
        .to_string();
    let refreshed_token = json
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or(refresh_token)
        .to_string();
    let expires_in = json
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .ok_or("No expires_in in response")?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    Ok(TokenExchange {
        access_token,
        refresh_token: refreshed_token,
        expires_at: now + expires_in,
    })
}

#[cfg(test)]
mod tests {
    use crate::auth::connectionstring::parse_device_code_connection_string;

    #[test]
    fn parses_device_code_connection_string() {
        let parsed = parse_device_code_connection_string(
            "AuthType=OAuth;Url=https://contosotest.crm.dynamics.com;AppId=51f81489-12ee-4a9e-aaae-a2591f45987d;RedirectUri=app://foo;LoginPrompt=Auto",
        )
        .expect("connection string should parse");

        assert_eq!(parsed.client_id, "51f81489-12ee-4a9e-aaae-a2591f45987d");
        assert_eq!(parsed.dataverse_url, "https://contosotest.crm.dynamics.com");
        assert_eq!(parsed.tenant_id, "organizations");
    }
}
