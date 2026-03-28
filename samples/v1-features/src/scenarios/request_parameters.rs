use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::dataverse::requestparameters::RequestParameters;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;
use serde_json::Value;
use uuid::Uuid;

pub fn run(connection_string: &str) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let client = ServiceClient::new(connection_string, LogLevel::Information).await?;
        let options = RequestParameters {
            bypass_business_logic_execution_custom_sync: true,
            bypass_business_logic_execution_custom_async: false,
            bypass_custom_plugin_execution: false,
            suppress_callback_registration_expander_job: false,
        };

        println!("Request parameter headers:");
        for (name, value) in options.headers() {
            println!("  {name}: {value}");
        }

        let suffix = Uuid::new_v4().simple().to_string();
        let mut create_attributes = HashMap::<String, Value>::new();
        create_attributes.insert(
            "name".to_string(),
            Value::String(format!("v1-features request-parameters {suffix}")),
        );

        let created = match client
            .create_entity_with_options("accounts", &create_attributes, &options)
            .await
        {
            Ok(id) => id,
            Err(error) => {
                println!(
                    "Create with request parameters failed: {error}\nThis usually means the Dataverse user lacks permission for the selected bypass header."
                );
                return Ok(());
            }
        };

        let Some(id) = created else {
            println!("Create succeeded but did not return an entity id.");
            return Ok(());
        };

        let mut update_attributes = HashMap::<String, Value>::new();
        update_attributes.insert(
            "tickersymbol".to_string(),
            Value::String("REQPARAM".to_string()),
        );

        if let Err(error) = client
            .update_entity_with_options("accounts", &id.to_string(), &update_attributes, &options)
            .await
        {
            println!("Update with request parameters failed: {error}");
            let _ = client.delete_entity("accounts", &id.to_string()).await;
            return Ok(());
        }

        if let Err(error) = client
            .delete_entity_with_options("accounts", &id.to_string(), &options)
            .await
        {
            println!("Delete with request parameters failed: {error}");
            let _ = client.delete_entity("accounts", &id.to_string()).await;
            return Ok(());
        }

        println!("Request parameters scenario completed successfully for account: {id}");
        Ok(())
    })
}
