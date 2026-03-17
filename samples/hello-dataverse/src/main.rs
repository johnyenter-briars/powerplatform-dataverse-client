mod config;
use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

use config::load_secrets;

#[tokio::main]
async fn main() -> Result<(), String> {
    let secrets = load_secrets()?;
    if secrets.connection_string.trim().is_empty() {
        return Err("Provide connection_string in secrets.json".to_string());
    }

    println!("Connecting to Dataverse...");
    let client = ServiceClient::new(&secrets.connection_string, LogLevel::Information).await?;
    let definitions = client.list_entity_definitions().await?;

    println!("Connected successfully.");
    println!("Entity definitions available: {}", definitions.len());
    Ok(())
}
