use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

use crate::config::Secrets;

pub async fn run(client: &ServiceClient, secrets: &Secrets) -> Result<(), String> {
    println!("Scenario: metadata");

    let definitions = client.list_entity_definitions().await?;
    println!("Entity definitions: {}", definitions.len());

    let logical = secrets.sample_entity_logical.as_str();
    let attributes = client.list_entity_attributes(logical).await?;
    println!("Attributes for {}: {}", logical, attributes.len());

    Ok(())
}
