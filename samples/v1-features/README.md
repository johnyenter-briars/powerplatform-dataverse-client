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
- `client-credentials-auth`
- `metadata`
- `fetchxml`
- `refresh-demo`

The `metadata` scenario exercises:

- `list_entity_definitions`
- `list_entity_attributes`
- `list_entity_relationships`
