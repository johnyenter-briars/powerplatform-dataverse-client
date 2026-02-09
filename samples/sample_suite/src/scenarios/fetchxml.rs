use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

pub async fn run(client: &ServiceClient) -> Result<(), String> {
    println!("Scenario: fetchxml");

    let accounts_fetchxml = r#"
        <fetch top="5">
        <entity name="account">
            <attribute name="accountid" />
            <attribute name="name" />
        </entity>
        </fetch>
    "#;

    let contacts_fetchxml = r#"
        <fetch top="5">
        <entity name="contact">
            <attribute name="contactid" />
            <attribute name="fullname" />
        </entity>
        </fetch>
    "#;

    run_fetchxml(client, "accounts", accounts_fetchxml).await?;
    run_fetchxml(client, "contacts", contacts_fetchxml).await?;

    Ok(())
}

async fn run_fetchxml(
    client: &ServiceClient,
    entity_set: &str,
    fetchxml: &str,
) -> Result<(), String> {
    let entities = client
        .retrieve_multiple_fetchxml(entity_set, fetchxml)
        .await?;

    println!("FetchXML [{}] returned {} record(s)", entity_set, entities.len());

    if let Some(first) = entities.first() {
        let mut keys = first
            .attributes
            .keys()
            .cloned()
            .collect::<Vec<String>>();

        keys.sort();

        println!("First record attributes: {}", keys.join(", "));
    }

    let count = client
        .retrieve_multiple_fetchxml_count(entity_set, fetchxml)
        .await?;

    println!("FetchXML [{}] count: {}", entity_set, count);

    Ok(())
}
