use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

pub async fn run(client: &ServiceClient) -> Result<(), String> {
    println!("Scenario: metadata");

    let definitions = client.list_entity_definitions().await?;
    println!("Entity definitions: {}", definitions.len());

    let attributes = client.list_entity_attributes("account").await?;
    println!("Attributes for account: {}", attributes.len());

    Ok(())
}
