use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::dataverse::batch::{
    CreateRequest, DeleteRequest, ExecuteMultipleRequest, ExecuteMultipleSettings,
    OrganizationRequest, OrganizationResponse, UpdateRequest,
};
use powerplatform_dataverse_client::dataverse::entity::{Entity, EntityReference, Value};
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;
use uuid::Uuid;

pub fn run(connection_string: &str) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let client = ServiceClient::new(connection_string, LogLevel::Information).await?;
        let suffix = Uuid::new_v4().simple().to_string();
        let create_requests = (0..2)
            .map(|index| {
                let mut entity = Entity::new(Uuid::nil(), "account", None);
                entity.attributes.insert(
                    "name".to_string(),
                    Value::String(format!("v1-features batch {index} {suffix}")),
                );
                entity.attributes.insert(
                    "tickersymbol".to_string(),
                    Value::String("BATCH".to_string()),
                );
                OrganizationRequest::Create(CreateRequest::new(entity))
            })
            .collect::<Vec<OrganizationRequest>>();

        let create_response = client
            .execute_multiple(&ExecuteMultipleRequest {
                settings: ExecuteMultipleSettings {
                    continue_on_error: false,
                    return_responses: true,
                },
                requests: create_requests,
            })
            .await?;

        let created_ids = create_response
            .responses
            .iter()
            .filter_map(|item| match &item.response {
                Some(OrganizationResponse::Create(response)) => response.id,
                _ => None,
            })
            .collect::<Vec<Uuid>>();

        if created_ids.len() != 2 {
            return Err(format!(
                "Batch create expected 2 created ids but got {}",
                created_ids.len()
            ));
        }

        println!("Batch create succeeded for {} records.", created_ids.len());

        let update_requests = created_ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let mut entity = Entity::new(*id, "account", None);
                entity.attributes.insert(
                    "tickersymbol".to_string(),
                    Value::String(format!("BATCH{index}")),
                );
                OrganizationRequest::Update(UpdateRequest::new(entity))
            })
            .collect::<Vec<OrganizationRequest>>();

        let update_response = client
            .execute_multiple(&ExecuteMultipleRequest {
                settings: ExecuteMultipleSettings {
                    continue_on_error: true,
                    return_responses: true,
                },
                requests: update_requests,
            })
            .await?;

        println!(
            "Batch update completed with {} response item(s).",
            update_response.responses.len()
        );

        let delete_requests = created_ids
            .iter()
            .map(|id| {
                OrganizationRequest::Delete(DeleteRequest::new(EntityReference {
                    id: *id,
                    logical_name: "account".to_string(),
                    name: None,
                }))
            })
            .collect::<Vec<OrganizationRequest>>();

        let delete_response = client
            .execute_multiple(&ExecuteMultipleRequest {
                settings: ExecuteMultipleSettings {
                    continue_on_error: true,
                    return_responses: true,
                },
                requests: delete_requests,
            })
            .await?;

        let delete_faults = delete_response
            .responses
            .iter()
            .filter(|item| item.fault.is_some())
            .count();

        println!(
            "Batch scenario complete. Created {}, updated {}, deleted {}. Faults: {}",
            created_ids.len(),
            created_ids.len(),
            created_ids.len(),
            delete_faults
        );

        Ok(())
    })
}
