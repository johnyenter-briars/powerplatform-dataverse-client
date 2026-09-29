# Device Code Auth

Device code auth is supported through Dataverse-style OAuth connection strings and through `AuthConfig::DeviceCode`.

## Connection String

```text
AuthType=OAuth;Url=https://YOUR_ORG.crm.dynamics.com;AppId=51f81489-12ee-4a9e-aaae-a2591f45987d;TokenCacheStorePath=C:\MyTokenCache\device-code-token.json
```

## Notes

- The client prints the Microsoft verification URL and device code to the console.
- After browser sign-in completes, the client receives the token and proceeds.
- Refresh tokens are persisted in the cache and used for automatic refresh.
- `RedirectUri` and `LoginPrompt` are not used by this device-code implementation.

## Sample Scenario

See [`samples/v1-features/src/scenarios/device_code_auth.rs`](../samples/v1-features/src/scenarios/device_code_auth.rs).
