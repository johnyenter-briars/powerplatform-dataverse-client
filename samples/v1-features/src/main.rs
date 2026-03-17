mod config;
mod scenarios;

use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

use config::Secrets;
use config::load_secrets;

type ScenarioFn =
    for<'a> fn(&'a ServiceClient) -> Pin<Box<dyn Future<Output = Result<(), String>> + 'a>>;

struct Scenario {
    name: &'static str,
    connection_string: fn(&Secrets) -> &str,
    run: ScenarioFn,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let secrets = load_secrets()?;

    let scenarios = [
        Scenario {
            name: "device code auth",
            connection_string: |secrets| &secrets.device_code_connection_string,
            run: scenarios::device_code_auth::run,
        },
        Scenario {
            name: "client credentials auth",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::client_credentials_auth::run,
        },
        Scenario {
            name: "metadata",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::metadata::run,
        },
        Scenario {
            name: "fetchxml",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::fetchxml::run,
        },
        Scenario {
            name: "refresh demo",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::refresh_demo::run,
        },
    ];

    let mut attempted = false;
    for scenario in scenarios {
        let connection_string = (scenario.connection_string)(&secrets);
        if connection_string.trim().is_empty() {
            println!(
                "Skipping {}: no connection string configured.",
                scenario.name
            );
            continue;
        }

        attempted = true;
        run_scenario(scenario.name, connection_string, scenario.run).await?;
    }

    if !attempted {
        return Err(
            "Provide device_code_connection_string and/or client_credentials_connection_string in secrets.json"
                .to_string(),
        );
    }

    Ok(())
}

async fn run_scenario(
    name: &str,
    connection_string: &str,
    scenario: ScenarioFn,
) -> Result<(), String> {
    println!("Running scenario: {name}");
    let client = ServiceClient::new(connection_string, LogLevel::Information).await?;
    scenario(&client).await?;
    Ok(())
}
