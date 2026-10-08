use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Url};
use ring::digest::{SHA256, digest};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{Duration, timeout};
use uuid::Uuid;

use crate::auth::credentials::TokenExchange;

const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: u64,
}

pub(crate) async fn fetch_interactive_browser_token(
    client_id: &str,
    dataverse_url: &str,
    tenant_id: &str,
) -> Result<TokenExchange, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| format!("Unable to start local sign-in callback: {e}"))?;
    let redirect_uri = format!(
        "http://localhost:{}",
        listener.local_addr().map_err(|e| e.to_string())?.port()
    );
    let verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let challenge = URL_SAFE_NO_PAD.encode(digest(&SHA256, verifier.as_bytes()).as_ref());
    let state = Uuid::new_v4().to_string();
    let scope = format!(
        "{}/user_impersonation offline_access openid profile",
        dataverse_url.trim_end_matches('/')
    );
    let tenant = if tenant_id.trim().is_empty() {
        "organizations"
    } else {
        tenant_id
    };

    let authorize_url =
        build_authorize_url(tenant, client_id, &redirect_uri, &scope, &challenge, &state)?;

    println!("Opening your default browser to choose a sign-in account...");
    open_browser(authorize_url.as_str())?;
    let code = timeout(SIGN_IN_TIMEOUT, receive_code(&listener, &state))
        .await
        .map_err(|_| "Browser sign-in timed out after five minutes".to_string())??;

    let token_url = format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token");
    let response = Client::new()
        .post(token_url)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("code_verifier", verifier.as_str()),
            ("scope", scope.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("Authorization code exchange failed: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Authorization code exchange failed ({}): {}",
            response.status(),
            response.text().await.unwrap_or_default()
        ));
    }
    let token: TokenResponse = response.json().await.map_err(|e| e.to_string())?;
    if token.access_token.is_empty() || token.refresh_token.is_empty() {
        return Err("Sign-in response did not include access and refresh tokens".to_string());
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    Ok(TokenExchange {
        access_token: token.access_token,
        refresh_token: token.refresh_token,
        expires_at: now + token.expires_in,
    })
}

fn build_authorize_url(
    tenant: &str,
    client_id: &str,
    redirect_uri: &str,
    scope: &str,
    challenge: &str,
    state: &str,
) -> Result<Url, String> {
    let mut authorize_url = Url::parse(&format!(
        "https://login.microsoftonline.com/{tenant}/oauth2/v2.0/authorize"
    ))
    .map_err(|e| format!("Invalid tenant ID: {e}"))?;
    authorize_url
        .query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("response_type", "code")
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_mode", "query")
        .append_pair("scope", &scope)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state)
        .append_pair("prompt", "select_account");
    Ok(authorize_url)
}

async fn receive_code(listener: &TcpListener, expected_state: &str) -> Result<String, String> {
    loop {
        let (mut stream, _) = listener.accept().await.map_err(|e| e.to_string())?;
        let request = match timeout(Duration::from_secs(10), read_request(&mut stream)).await {
            Ok(Ok(request)) => request,
            _ => continue,
        };
        let Some(path) = request.split_whitespace().nth(1) else {
            continue;
        };
        let Ok(url) = Url::parse(&format!("http://localhost{path}")) else {
            continue;
        };
        if !request.starts_with("GET ") || url.path() != "/" {
            continue;
        }
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        if params.get("state").map(String::as_str) != Some(expected_state) {
            send_response(
                &mut stream,
                "400 Bad Request",
                "Sign-in state did not match.",
            )
            .await;
            continue;
        }
        if let Some(error) = params.get("error") {
            send_response(
                &mut stream,
                "200 OK",
                "Sign-in was not completed. Return to the app.",
            )
            .await;
            return Err(format!(
                "Browser sign-in failed: {error}: {}",
                params
                    .get("error_description")
                    .map(String::as_str)
                    .unwrap_or("")
            ));
        }
        if let Some(code) = params.get("code").filter(|code| !code.is_empty()) {
            send_response(
                &mut stream,
                "200 OK",
                "Sign-in received. You can return to the app.",
            )
            .await;
            return Ok(code.clone());
        }
        send_response(
            &mut stream,
            "400 Bad Request",
            "Authorization code missing.",
        )
        .await;
    }
}

async fn read_request(stream: &mut TcpStream) -> Result<String, String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    while buffer.len() < 8192 {
        let count = stream.read(&mut chunk).await.map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..count]);
        if buffer.windows(4).any(|part| part == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8(buffer).map_err(|e| e.to_string())
}

async fn send_response(stream: &mut TcpStream, status: &str, message: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{message}",
        message.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let result = Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(url).spawn();
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(url).spawn();
    result.map(|_| ()).map_err(|e| {
        format!("Could not open the sign-in browser ({e}). Open this URL manually: {url}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sign_in_requests_account_selection() {
        let url = build_authorize_url(
            "organizations",
            "client-id",
            "http://localhost:12345",
            "https://example.crm.dynamics.com/user_impersonation offline_access",
            "challenge",
            "state",
        )
        .unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(
            params.get("prompt").map(String::as_str),
            Some("select_account")
        );
        assert_eq!(
            params.get("redirect_uri").map(String::as_str),
            Some("http://localhost:12345")
        );
        assert_eq!(params.get("state").map(String::as_str), Some("state"));
    }

    #[tokio::test]
    async fn accepts_only_matching_callback_state() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let receiver = tokio::spawn(async move { receive_code(&listener, "expected").await });
        let client = Client::new();
        let invalid = client
            .get(format!("http://{address}/?code=wrong&state=invalid"))
            .send()
            .await
            .unwrap();
        assert_eq!(invalid.status(), reqwest::StatusCode::BAD_REQUEST);
        let valid = client
            .get(format!("http://{address}/?code=good&state=expected"))
            .send()
            .await
            .unwrap();
        assert_eq!(valid.status(), reqwest::StatusCode::OK);
        assert_eq!(receiver.await.unwrap().unwrap(), "good");
    }
}
