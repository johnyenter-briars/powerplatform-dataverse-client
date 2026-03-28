# Authentication

Authentication in this crate is centered on `AuthConfig`, device-code progress reporting, token refresh, and token caching.

Microsoft Learn background:

- [Authenticate with Dataverse Web API](https://learn.microsoft.com/power-apps/developer/data-platform/authenticate-oauth)

Related detailed pages:

- [Client credentials auth](client-credentials-auth.md)
- [Device code auth](device-code-auth.md)
- [Token refresh](token-refresh.md)
- [Token cache](token-cache.md)

## Public API

### `AuthConfig`

Public enum used by `ServiceClient::new_with_auth`.

Variants:

- `AuthConfig::ClientCredentials`
- `AuthConfig::DeviceCode`

Methods:

- `AuthConfig::from_connection_string(connection_string: &str) -> Result<AuthConfig, String>`

### `DeviceCodeFlowEvent`

Progress event enum emitted during device-code sign-in.

Variants:

- `Code { verification_uri, verification_uri_complete, user_code, message }`
- `Waiting`
- `Success`

### `ensure_device_code_token_with_progress`

```rust
pub async fn ensure_device_code_token_with_progress<F>(
    auth: &AuthConfig,
    progress: F,
) -> Result<(), String>
where
    F: Fn(DeviceCodeFlowEvent) + Send + Sync
```

Ensures a cached device-code token exists, emitting progress events when sign-in is needed.

### Token result types

The crate also exposes:

- `TokenExchange`
- `ClientCredentialsToken`

These types are public because they are part of the crate's auth surface, even though most callers use `ServiceClient` and do not construct them directly.

## Notes

- `ServiceClient::new(...)` is usually the simplest auth entry point.
- `AuthConfig::from_connection_string(...)` is useful when a caller wants to inspect or reuse the parsed auth model before constructing a client.
- Device-code flows can be fully interactive through `ensure_device_code_token_with_progress(...)`, which is what the `v1-features` device-code progress sample demonstrates.
