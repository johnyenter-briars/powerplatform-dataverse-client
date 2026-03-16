use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

pub fn run(client: &ServiceClient) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let definitions = client.list_entity_definitions().await?;
        println!("Entity definitions: {}", definitions.len());

        let attributes = client.list_entity_attributes("account").await?;
        println!("Attributes for account: {}", attributes.len());
        let relationships = client.list_entity_relationships("account").await?;
        println!("Relationships for account: {}", relationships.len());

        let attributes = client.list_entity_attributes("activitypointer").await?;
        println!("Attributes for activitypointer: {:?}", attributes.len());
        let relationships = client.list_entity_relationships("activitypointer").await?;
        println!("Relationships for activitypointer: {:?}", relationships.len());

        let attributes = client.list_entity_attributes("email").await?;
        println!("Attributes for email: {:?}", attributes.len());
        let relationships = client.list_entity_relationships("email").await?;
        println!("Relationships for email: {:?}", relationships.len());

        Ok(())
    })
}
