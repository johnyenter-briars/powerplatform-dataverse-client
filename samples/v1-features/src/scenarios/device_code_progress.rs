use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::auth::config::AuthConfig;
use powerplatform_dataverse_client::auth::devicecode::{
    DeviceCodeFlowEvent, ensure_device_code_token_with_progress,
};
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

pub fn run(connection_string: &str) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let auth = AuthConfig::from_connection_string(connection_string)?;
        ensure_device_code_token_with_progress(&auth, |event| match event {
            DeviceCodeFlowEvent::Code {
                verification_uri,
                verification_uri_complete,
                user_code,
                message,
            } => {
                println!("Device code prompt:");
                println!("  verification_uri: {verification_uri}");
                if let Some(verification_uri_complete) = verification_uri_complete {
                    println!("  verification_uri_complete: {verification_uri_complete}");
                }
                println!("  user_code: {user_code}");
                if let Some(message) = message {
                    println!("  message: {message}");
                }
            }
            DeviceCodeFlowEvent::Waiting => println!("Waiting for browser sign-in..."),
            DeviceCodeFlowEvent::Success => println!("Device code authentication completed."),
        })
        .await?;

        let client = ServiceClient::new(connection_string, LogLevel::Information).await?;
        let definitions = client.list_entity_definitions().await?;
        println!(
            "Device code progress scenario complete. Entity definitions: {}",
            definitions.len()
        );

        Ok(())
    })
}
