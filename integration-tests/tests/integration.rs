use powerplatform_dataverse_client::auth::credentials::fetch_client_credentials_token;
use powerplatform_dataverse_client::dataverse::entity::Value;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;
use powerplatform_dataverse_client::LogLevel;

use powerplatform_dataverse_integration_tests::config::load_secrets;

async fn create_client() -> Result<ServiceClient, String> {
    let secrets = load_secrets()?;

    let token = fetch_client_credentials_token(
        &secrets.client_id,
        &secrets.client_secret,
        &secrets.tenant_id,
        &secrets.scope,
    )
    .await?;

    Ok(ServiceClient::new(
        &secrets.dataverse_url,
        &token,
        LogLevel::Information,
    ))
}

#[tokio::test]
async fn metadata_smoke() -> Result<(), String> {
    let client = create_client().await?;

    let definitions = client.list_entity_definitions().await?;
    assert!(
        !definitions.is_empty(),
        "Expected at least one entity definition"
    );

    let first = &definitions[0];
    assert!(!first.logical_name.trim().is_empty());
    assert!(!first.schema_name.trim().is_empty());
    assert!(!first.entity_set_name.trim().is_empty());

    let attributes = client.list_entity_attributes("account").await?;
    assert!(
        !attributes.is_empty(),
        "Expected account attributes to be returned"
    );

    let has_account_id = attributes
        .iter()
        .any(|attr| attr.logical_name.eq_ignore_ascii_case("accountid"));
    assert!(
        has_account_id,
        "Expected account attributes to include accountid"
    );

    Ok(())
}

#[tokio::test]
async fn fetchxml_smoke() -> Result<(), String> {
    let client = create_client().await?;

    let accounts_fetchxml = r#"
        <fetch top="1">
          <entity name="account">
            <attribute name="accountid" />
            <attribute name="name" />
          </entity>
        </fetch>
    "#;

    let entities = client
        .retrieve_multiple_fetchxml("accounts", accounts_fetchxml)
        .await?;

    let count = client
        .retrieve_multiple_fetchxml_count("accounts", accounts_fetchxml)
        .await?;

    assert_eq!(
        entities.len(),
        count,
        "Expected count to match returned entities for top=1"
    );

    Ok(())
}
