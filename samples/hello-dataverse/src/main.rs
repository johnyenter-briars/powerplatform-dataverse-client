mod config;
mod scenarios;

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
        let client =
            ServiceClient::new(&secrets.device_code_connection_string, LogLevel::Information)
                .await?;
        run_scenarios("connection string", &client).await?;
    }

    if !secrets.client_credentials_connection_string.trim().is_empty() {
        attempted = true;
        println!("Authenticating with client-credentials connection string...");
        let client = ServiceClient::new(
            &secrets.client_credentials_connection_string,
            LogLevel::Information,
        )
        .await?;
        run_scenarios("client credentials connection string", &client).await?;
    }

    if !attempted {
        return Err(
            "Provide device_code_connection_string and/or client_credentials_connection_string in secrets.json"
                .to_string(),
        );
    }

    Ok(())
}

async fn run_scenarios(label: &str, client: &ServiceClient) -> Result<(), String> {
    println!("Running sample with {label}...");
    scenarios::metadata::run(client).await?;
    scenarios::fetchxml::run(client).await?;
    Ok(())
}
