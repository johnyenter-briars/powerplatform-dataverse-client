use std::future::Future;
use std::pin::Pin;

use crate::config::BUILTIN_SAMPLE_TABLES;
use powerplatform_dataverse_client::LogLevel;
use powerplatform_dataverse_client::dataverse::entity::Value;
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

type SupportedType = (&'static str, &'static [&'static str]);

const SUPPORTED_TYPES: &[SupportedType] = &[
    ("BigInt", &["BigInt", "BigIntType"]),
    ("Boolean", &["Boolean", "BooleanType"]),
    ("DateTime", &["DateTime", "DateTimeType"]),
    ("Decimal", &["Decimal", "DecimalType"]),
    ("Double", &["Double", "DoubleType"]),
    ("Guid", &["Guid", "Uniqueidentifier", "UniqueidentifierType"]),
    ("Integer", &["Integer", "IntegerType"]),
    ("Money", &["Money", "MoneyType"]),
    ("MultiSelectPicklist", &["MultiSelectPicklist", "MultiSelectPicklistType"]),
    ("OptionSet", &[
        "Picklist",
        "PicklistType",
        "State",
        "StateType",
        "Status",
        "StatusType",
    ]),
    ("String", &[
        "String",
        "StringType",
        "Memo",
        "MemoType",
        "EntityName",
        "EntityNameType",
    ]),
    ("EntityReference", &[
        "Customer",
        "CustomerType",
        "Lookup",
        "LookupType",
        "Owner",
        "OwnerType",
    ]),
];

pub fn run(connection_string: &str) -> Pin<Box<dyn Future<Output = Result<(), String>> + '_>> {
    Box::pin(async move {
        let client = ServiceClient::new(connection_string, LogLevel::Information).await?;
        let definitions = client.list_entity_definitions().await?;
        let preferred_tables = BUILTIN_SAMPLE_TABLES
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect::<Vec<String>>();

        for (label, type_keys) in SUPPORTED_TYPES {
            let mut matched = false;

            'entities: for definition in &definitions {
                if !preferred_tables
                    .iter()
                    .any(|table| table.eq_ignore_ascii_case(&definition.logical_name))
                {
                    continue;
                }

                let entity_name = if definition.entity_set_name.trim().is_empty() {
                    continue;
                } else {
                    &definition.entity_set_name
                };

                let attributes = client.list_entity_attributes(&definition.logical_name).await?;
                for attribute in attributes {
                    if !attribute_matches_type(&attribute, type_keys) {
                        continue;
                    }

                    let Some(primary_id) = definition.primary_id_attribute.as_deref() else {
                        continue;
                    };

                    let fetchxml = format!(
                        "<fetch top=\"1\"><entity name=\"{}\"><attribute name=\"{}\" /><attribute name=\"{}\" /><filter><condition attribute=\"{}\" operator=\"not-null\" /></filter></entity></fetch>",
                        definition.logical_name,
                        primary_id,
                        attribute.logical_name,
                        attribute.logical_name
                    );

                    println!(
                        "[{}] probing {}.{}",
                        label, definition.logical_name, attribute.logical_name
                    );

                    let entities = client
                        .retrieve_multiple_fetchxml(entity_name, &fetchxml)
                        .await?;

                    if let Some(entity) = entities.first()
                        && let Some(value) = entity.attributes.get(&attribute.logical_name)
                        && !matches!(value, Value::Null)
                    {
                        println!(
                            "[{}] {}.{} = {}",
                            label,
                            definition.logical_name,
                            attribute.logical_name,
                            format_value(value)
                        );
                        matched = true;
                        break 'entities;
                    }
                }
            }

            if !matched {
                println!("[{}] no non-null sample found", label);
            }
        }

        Ok(())
    })
}

fn attribute_matches_type(
    attribute: &powerplatform_dataverse_client::dataverse::entityattribute::EntityAttribute,
    keys: &[&str],
) -> bool {
    let specific = attribute
        .attribute_type_name
        .as_ref()
        .and_then(|value| value.value.as_deref());
    let basic = attribute.attribute_type.as_deref();

    keys.iter().any(|key| {
        specific.is_some_and(|value| value.eq_ignore_ascii_case(key))
            || basic.is_some_and(|value| value.eq_ignore_ascii_case(key))
    })
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Int(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::Decimal(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Boolean(value) => value.to_string(),
        Value::DateTime(value) => value.to_rfc3339(),
        Value::Guid(value) => value.to_string(),
        Value::Money(value) => value.value.to_string(),
        Value::OptionSetValue(value) => match &value.name {
            Some(name) => format!("{} ({})", value.value, name),
            None => value.value.to_string(),
        },
        Value::OptionSetValueCollection(value) => format!("{:?}", value.values),
        Value::Null => "null".to_string(),
        Value::EntityReference(reference) => format!(
            "{}:{} ({})",
            reference.logical_name,
            reference.id,
            reference.name.as_deref().unwrap_or("<no name>")
        ),
    }
}
