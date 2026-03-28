# Entity and Value Types

The crate exposes typed Dataverse row and value models so callers do not have to work only with raw JSON.

Microsoft Learn background:

- [Use the Microsoft Dataverse Web API](https://learn.microsoft.com/power-apps/developer/data-platform/webapi/overview)

## Public API

### Core row types

- `Attribute = String`
- `Entity`
- `EntityReference`

### Value types

- `Value`
- `Money`
- `OptionSetValue`
- `OptionSetValueCollection`

### Constructors

- `Entity::new(id: Uuid, logical_name: impl Into<String>, name: Option<String>) -> Entity`

## `Value` Variants

- `Value::Int(i64)`
- `Value::Float(f64)`
- `Value::Decimal(Decimal)`
- `Value::String(String)`
- `Value::Boolean(bool)`
- `Value::DateTime(DateTime<Utc>)`
- `Value::Guid(Uuid)`
- `Value::Money(Money)`
- `Value::OptionSetValue(OptionSetValue)`
- `Value::OptionSetValueCollection(OptionSetValueCollection)`
- `Value::Null`
- `Value::EntityReference(EntityReference)`

## Notes

- `Entity` is the typed row shape returned from FetchXML retrieval helpers.
- `EntityReference` is also used in batch delete operations.
- CRUD helpers that take plain `HashMap<String, serde_json::Value>` are intentionally lighter-weight than the typed `Entity` model; both styles are supported.
