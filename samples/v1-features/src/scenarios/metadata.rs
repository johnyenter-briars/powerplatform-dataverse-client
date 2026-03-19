use std::future::Future;
use std::pin::Pin;

use crate::config::BUILTIN_SAMPLE_TABLES;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

pub fn run(client: &ServiceClient) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let definitions = client.list_entity_definitions().await?;
        println!("Entity definitions: {}", definitions.len());

        for table in BUILTIN_SAMPLE_TABLES {
            let attributes = client.list_entity_attributes(table).await?;
            println!("Attributes for {}: {}", table, attributes.len());
            let relationships = client.list_entity_relationships(table).await?;
            println!("Relationships for {}: {}", table, relationships.len());
        }

        Ok(())
    })
}
