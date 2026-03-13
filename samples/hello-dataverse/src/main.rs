mod config;
mod scenarios;

use powerplatform_dataverse_client::auth::token::fetch_token as fetch_connection_string_token;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;
use powerplatform_dataverse_client::LogLevel;

use config::load_secrets;

#[tokio::main]
async fn main() -> Result<(), String> {
    let secrets = load_secrets()?;
    let mut attempted = false;

    if !secrets.device_code_connection_string.trim().is_empty() {
        attempted = true;
        println!("Authenticating with connection string...");
        let token = fetch_connection_string_token(&secrets.device_code_connection_string)
            .await?
            .access_token;
        run_scenarios("connection string", &secrets.dataverse_url, &token).await?;
    }

    if !secrets.client_credentials_connection_string.trim().is_empty() {
        attempted = true;
        println!("Authenticating with client-credentials connection string...");
        let token = fetch_connection_string_token(&secrets.client_credentials_connection_string)
            .await?
            .access_token;
        run_scenarios("client credentials connection string", &secrets.dataverse_url, &token)
            .await?;
    }

    if !attempted {
        return Err(
            "Provide device_code_connection_string and/or client_credentials_connection_string in secrets.json"
                .to_string(),
        );
    }

    Ok(())
}

async fn run_scenarios(label: &str, dataverse_url: &str, token: &str) -> Result<(), String> {
    println!("Running sample with {label}...");
    let client = ServiceClient::new(dataverse_url, token, LogLevel::Information);
    scenarios::metadata::run(&client).await?;
    scenarios::fetchxml::run(&client).await?;
    Ok(())
}
