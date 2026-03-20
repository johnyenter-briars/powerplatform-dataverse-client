use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::dataverse::batch::{
    ExecuteMultipleRequest, ExecuteMultipleSettings, OrganizationRequest, UpdateRequest,
};
use powerplatform_dataverse_client::dataverse::entity::{Entity, Value};
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

const MAX_RECORDS: usize = 10_000;
const BATCH_SIZE: usize = 200;

pub fn run(client: &ServiceClient) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let fetchxml = r#"
        <fetch>
            <entity name="account">
                <attribute name="accountid" />
                <attribute name="tickersymbol" />
            </entity>
        </fetch>
    "#;

        let mut accounts = client
            .retrieve_multiple_fetchxml_paging("accounts", fetchxml)
            .await?;
        if accounts.len() > MAX_RECORDS {
            accounts.truncate(MAX_RECORDS);
        }
        if accounts.is_empty() {
            println!("No account records found for batch scenario.");
            return Ok(());
        }

        println!(
            "Preparing {} account update requests in batches of {}...",
            accounts.len(),
            BATCH_SIZE
        );

        let requests = accounts
            .into_iter()
            .map(|account| {
                let mut target = Entity::new(account.id, "account", account.name.clone());
                target.attributes.insert(
                    "tickersymbol".to_string(),
                    Value::String("foo".to_string())
                );

                OrganizationRequest::Update(UpdateRequest::new(target))
            })
            .collect::<Vec<OrganizationRequest>>();

        let total_batches = requests.len().div_ceil(BATCH_SIZE);
        let mut total_faults = 0usize;

        for (batch_index, chunk) in requests.chunks(BATCH_SIZE).enumerate() {
            println!(
                "Executing batch {}/{} ({} request(s))",
                batch_index + 1,
                total_batches,
                chunk.len()
            );

            let response = client
                .execute_multiple(&ExecuteMultipleRequest {
                    settings: ExecuteMultipleSettings {
                        continue_on_error: false,
                        return_responses: false,
                    },
                    requests: chunk.to_vec(),
                })
                .await?;

            let batch_faults = response.responses.iter().filter(|item| item.fault.is_some()).count();
            total_faults += batch_faults;

            if batch_faults > 0 {
                for item in response.responses.iter().filter(|item| item.fault.is_some()) {
                    if let Some(fault) = &item.fault {
                        println!(
                            "Batch fault at request {}: {} ({})",
                            item.request_index,
                            fault.message,
                            fault.code.as_deref().unwrap_or("no code")
                        );
                    }
                }
            }
        }

        println!(
            "Batch scenario complete. Updated {} record(s) across {} batch(es). Faults: {}",
            requests.len(),
            total_batches,
            total_faults
        );

        Ok(())
    })
}
