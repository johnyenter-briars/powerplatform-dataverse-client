mod config;
mod scenarios;

use powerplatform_dataverse_client::auth::credentials::fetch_client_credentials_token;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;
use powerplatform_dataverse_client::LogLevel;

use config::load_secrets;

#[tokio::main]
async fn main() -> Result<(), String> {
    let secrets = load_secrets()?;

    let token = fetch_client_credentials_token(
        &secrets.client_id,
        &secrets.client_secret,
        &secrets.tenant_id,
        &secrets.scope,
    )
    .await?;

    let client = ServiceClient::new(&secrets.dataverse_url, &token, LogLevel::Information);

    scenarios::metadata::run(&client, &secrets).await?;
    scenarios::fetchxml::run(&client, &secrets).await?;

    Ok(())
}
