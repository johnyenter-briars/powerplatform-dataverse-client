use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

pub async fn run(client: &ServiceClient) -> Result<(), String> {
    println!("Scenario: refresh demo");
    println!(
        "Press Enter to make a request and print the token expiry. Type 'q' and press Enter to quit."
    );

    loop {
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|e| e.to_string())?;

        if input.trim().eq_ignore_ascii_case("q") {
            break;
        }

        let definitions = client.list_entity_definitions().await?;
        let expires_at = client
            .token_expires_at()
            .await
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| "unknown".to_string());

        println!(
            "Request succeeded. Entity definitions: {}. Token expires at: {}",
            definitions.len(),
            expires_at
        );
    }

    Ok(())
}
