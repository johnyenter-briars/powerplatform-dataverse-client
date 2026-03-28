# v1-features

The `v1-features` sample contains all the scenarios in the v1 version of the `ServiceClient`.

## Requirements

- `secrets.json` - see the attached `secrets.example.json` for an example format

## CLI

By default, the sample runs every scenario that has the required connection string configured in `secrets.json`.

### Options

- `--scenario <id>`: Run only the named scenario or scenarios.
- `--list-scenarios`: Print the available scenario ids and exit.
- `--help`, `-h`: Print usage information.

You can pass multiple scenario ids in either of these forms:

```bash
cargo run -- --scenario metadata --scenario fetchxml
```

```bash
cargo run -- --scenario metadata,fetchxml
```

### Available scenario ids

- `device-code-auth`
- `device-code-progress`
- `client-credentials-auth`
- `batch`
- `crud`
- `metadata`
- `data-types`
- `fetchxml`
- `request-parameters`
- `refresh-demo`

The `metadata` scenario exercises:

- `list_entity_definitions`
- `list_entity_attributes`
- `list_entity_relationships`

The `data-types` scenario scans metadata for supported Dataverse field types, finds the first non-null sample value it can retrieve for each type, and prints the typed Rust value.

The `batch` scenario demonstrates `ExecuteMultiple` create, update, and delete requests with returned responses enabled.

The `crud` scenario demonstrates direct `create_entity`, `update_entity`, and `delete_entity` calls against a temporary account row.

The `request-parameters` scenario demonstrates the `*_with_options` methods and prints the Dataverse headers represented by `RequestParameters`.

The `device-code-progress` scenario exercises `ensure_device_code_token_with_progress` and prints the emitted progress events before constructing a `ServiceClient`.
