use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

pub fn run(connection_string: &str) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let client = ServiceClient::new(connection_string, LogLevel::Information).await?;
        let definitions = client.list_entity_definitions().await?;
        let expires_at = client
            .token_expires_at()
            .await
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| "unknown".to_string());

        println!(
            "Client credentials auth succeeded. Entity definitions: {}. Token expires at: {}",
            definitions.len(),
            expires_at
        );

        Ok(())
    })
}
