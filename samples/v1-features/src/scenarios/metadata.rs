use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

pub fn run(client: &ServiceClient) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let definitions = client.list_entity_definitions().await?;
        println!("Entity definitions: {}", definitions.len());

        let attributes = client.list_entity_attributes("account").await?;
        println!("Attributes for account: {}", attributes.len());

        Ok(())
    })
}
