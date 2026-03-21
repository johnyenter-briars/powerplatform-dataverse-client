mod config;
mod scenarios;

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;

use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

use config::Secrets;
use config::load_secrets;

type ScenarioFn =
    for<'a> fn(&'a ServiceClient) -> Pin<Box<dyn Future<Output = Result<(), String>> + 'a>>;

struct Scenario {
    id: &'static str,
    name: &'static str,
    connection_string: fn(&Secrets) -> &str,
    run: ScenarioFn,
}

#[derive(Default)]
struct CliArgs {
    selected_scenarios: Vec<String>,
    list_scenarios: bool,
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let scenarios = [
        Scenario {
            id: "batch",
            name: "batch",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::batch::run,
        },
        Scenario {
            id: "device-code-auth",
            name: "device code auth",
            connection_string: |secrets| &secrets.device_code_connection_string,
            run: scenarios::device_code_auth::run,
        },
        Scenario {
            id: "client-credentials-auth",
            name: "client credentials auth",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::client_credentials_auth::run,
        },
        Scenario {
            id: "metadata",
            name: "metadata",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::metadata::run,
        },
        Scenario {
            id: "data-types",
            name: "data types",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::data_types::run,
        },
        Scenario {
            id: "fetchxml",
            name: "fetchxml",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::fetchxml::run,
        },
        Scenario {
            id: "refresh-demo",
            name: "refresh demo",
            connection_string: |secrets| &secrets.client_credentials_connection_string,
            run: scenarios::refresh_demo::run,
        },
    ];

    let cli = parse_cli_args()?;
    if cli.list_scenarios {
        print_scenarios(&scenarios);
        return Ok(());
    }

    let selected = validate_selected_scenarios(&scenarios, &cli.selected_scenarios)?;
    let secrets = load_secrets()?;
    let mut attempted = false;
    for scenario in scenarios {
        if !selected.is_empty() && !selected.contains(scenario.id) {
            continue;
        }

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
        return if selected.is_empty() {
            Err(
                "Provide device_code_connection_string and/or client_credentials_connection_string in secrets.json"
                    .to_string(),
            )
        } else {
            Err("No selected scenarios could run with the configured connection strings.".to_string())
        };
    }

    Ok(())
}

fn parse_cli_args() -> Result<CliArgs, String> {
    let mut cli = CliArgs::default();
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scenario" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--scenario requires a value".to_string())?;
                cli.selected_scenarios.extend(split_scenario_values(&value));
            }
            "--list-scenarios" => cli.list_scenarios = true,
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                if let Some(value) = arg.strip_prefix("--scenario=") {
                    cli.selected_scenarios.extend(split_scenario_values(value));
                } else {
                    return Err(format!("Unknown argument: {arg}"));
                }
            }
        }
    }

    Ok(cli)
}

fn split_scenario_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

fn validate_selected_scenarios(
    scenarios: &[Scenario],
    selected: &[String],
) -> Result<HashSet<&'static str>, String> {
    let valid_ids: HashSet<&'static str> = scenarios.iter().map(|scenario| scenario.id).collect();
    let mut validated = HashSet::new();

    for scenario in selected {
        if !valid_ids.contains(scenario.as_str()) {
            return Err(format!(
                "Unknown scenario '{scenario}'. Use --list-scenarios to see valid values."
            ));
        }
        validated.insert(
            scenarios
                .iter()
                .find(|candidate| candidate.id == scenario)
                .map(|candidate| candidate.id)
                .expect("validated scenario id should exist"),
        );
    }

    Ok(validated)
}

fn print_scenarios(scenarios: &[Scenario]) {
    println!("Available scenarios:");
    for scenario in scenarios {
        println!("  {} ({})", scenario.id, scenario.name);
    }
}

fn print_usage() {
    println!("Usage: cargo run -- [--scenario <id>[,<id>...]] [--list-scenarios]");
    println!();
    println!("Options:");
    println!("  --scenario <id>         Run one or more specific scenarios.");
    println!("                          Repeat the flag or pass a comma-separated list.");
    println!("  --list-scenarios        Print the available scenario ids and exit.");
    println!("  --help, -h              Show this help text.");
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
