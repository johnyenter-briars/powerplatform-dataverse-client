use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;
use serde_json::Value;
use uuid::Uuid;

pub fn run(connection_string: &str) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let client = ServiceClient::new(connection_string, LogLevel::Information).await?;
        let suffix = Uuid::new_v4().simple().to_string();

        let mut create_attributes = HashMap::<String, Value>::new();
        create_attributes.insert(
            "name".to_string(),
            Value::String(format!("v1-features crud {suffix}")),
        );
        create_attributes.insert("tickersymbol".to_string(), Value::String("CRUD".to_string()));

        let id = client
            .create_entity("accounts", &create_attributes)
            .await?
            .ok_or("Create did not return an entity id".to_string())?;

        println!("Created account: {id}");

        let mut update_attributes = HashMap::<String, Value>::new();
        update_attributes.insert(
            "tickersymbol".to_string(),
            Value::String("CRUDUPD".to_string()),
        );

        client
            .update_entity("accounts", &id.to_string(), &update_attributes)
            .await?;

        println!("Updated account: {id}");

        client.delete_entity("accounts", &id.to_string()).await?;

        println!("Deleted account: {id}");
        Ok(())
    })
}
