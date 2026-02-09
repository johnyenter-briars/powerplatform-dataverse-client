use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

use crate::config::Secrets;

pub async fn run(client: &ServiceClient, secrets: &Secrets) -> Result<(), String> {
    println!("Scenario: fetchxml");

    let entities = client
        .retrieve_multiple_fetchxml(&secrets.sample_entity_set, &secrets.sample_fetchxml)
        .await?;
    println!("FetchXML returned {} record(s)", entities.len());

    if let Some(first) = entities.first() {
        let keys = first
            .attributes
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        println!("First record attributes: {}", keys.join(", "));
    }

    let count = client
        .retrieve_multiple_fetchxml_count(&secrets.sample_entity_set, &secrets.sample_fetchxml)
        .await?;
    println!("FetchXML count: {}", count);

    Ok(())
}
