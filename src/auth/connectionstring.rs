use std::collections::HashMap;

use super::config::AuthConfig;

pub(crate) struct DeviceCodeConnectionString {
    pub(crate) client_id: String,
    pub(crate) dataverse_url: String,
    pub(crate) tenant_id: String,
    pub(crate) token_cache_store_path: Option<String>,
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

pub(crate) fn parse_connection_string_auth_config(
    connection_string: &str,
) -> Result<AuthConfig, String> {
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

    let interactive = values
        .get("authtype")
        .is_some_and(|value| value.eq_ignore_ascii_case("interactivebrowser"))
        || values
            .get("authenticationflow")
            .is_some_and(|value| value.eq_ignore_ascii_case("interactivebrowser"));
    if interactive {
        let client_id =
            client_id.ok_or("Interactive browser auth requires AppId or ClientId".to_string())?;
        return Ok(AuthConfig::InteractiveBrowser {
            client_id,
            dataverse_url: url.trim_end_matches('/').to_string(),
            tenant_id: tenant_id.unwrap_or_else(|| "organizations".to_string()),
            token_cache_store_path,
        });
    }

    if let (Some(client_id), Some(client_secret), Some(tenant_id)) =
        (client_id.clone(), client_secret, tenant_id.clone())
    {
        return Ok(AuthConfig::ClientCredentials {
            client_id,
            client_secret,
            tenant_id,
            dataverse_url: url.trim_end_matches('/').to_string(),
            token_cache_store_path,
        });
    }

    let parsed = parse_device_code_connection_string(connection_string)?;

    Ok(AuthConfig::DeviceCode {
        client_id: parsed.client_id,
        dataverse_url: parsed.dataverse_url,
        tenant_id: parsed.tenant_id,
        token_cache_store_path: parsed.token_cache_store_path,
    })
}

pub(crate) fn parse_connection_string_url(connection_string: &str) -> Result<String, String> {
    let values = parse_connection_string_values(connection_string)?;
    let url = values
        .get("url")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or("Connection string missing Url".to_string())?;

    Ok(url.trim_end_matches('/').to_string())
}

pub(crate) fn parse_device_code_connection_string(
    connection_string: &str,
) -> Result<DeviceCodeConnectionString, String> {
    let values = parse_connection_string_values(connection_string)?;

    let auth_type = values
        .get("authtype")
        .ok_or("Connection string missing AuthType".to_string())?;
    if !auth_type.eq_ignore_ascii_case("oauth") {
        return Err("Device code auth requires AuthType=OAuth".to_string());
    }

    let client_id = values
        .get("appid")
        .cloned()
        .ok_or("Connection string missing AppId".to_string())?;
    let dataverse_url = values
        .get("url")
        .cloned()
        .ok_or("Connection string missing Url".to_string())?;

    let tenant_id = values
        .get("tenantid")
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "organizations".to_string());
    let token_cache_store_path = values
        .get("tokencachestorepath")
        .cloned()
        .filter(|value| !value.trim().is_empty());

    if client_id.trim().is_empty() {
        return Err("Connection string AppId was empty".to_string());
    }

    if dataverse_url.trim().is_empty() {
        return Err("Connection string Url was empty".to_string());
    }

    Ok(DeviceCodeConnectionString {
        client_id,
        dataverse_url: dataverse_url.trim_end_matches('/').to_string(),
        tenant_id,
        token_cache_store_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_browser_without_changing_oauth_default() {
        let base = "AuthType=OAuth;Url=https://example.crm.dynamics.com;AppId=client";
        assert!(matches!(
            parse_connection_string_auth_config(base).unwrap(),
            AuthConfig::DeviceCode { .. }
        ));
        let browser = format!("{base};AuthenticationFlow=InteractiveBrowser");
        assert!(matches!(
            parse_connection_string_auth_config(&browser).unwrap(),
            AuthConfig::InteractiveBrowser { .. }
        ));
        let browser =
            "AuthType=InteractiveBrowser;Url=https://example.crm.dynamics.com;ClientId=client";
        assert!(matches!(
            parse_connection_string_auth_config(browser).unwrap(),
            AuthConfig::InteractiveBrowser { .. }
        ));
    }
}
