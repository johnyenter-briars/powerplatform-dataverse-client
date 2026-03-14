# Device Code Auth

Device code auth is supported through Dataverse-style OAuth connection strings and through `AuthConfig::DeviceCode`.

## Connection String

```text
AuthType=OAuth;Url=https://YOUR_ORG.crm.dynamics.com;AppId=APP_ID;RedirectUri=app://58145B91-0C36-4500-8554-080854F2AC97;LoginPrompt=Auto;TokenCacheStorePath=C:\MyTokenCache
```

## Notes

- The client prints the Microsoft verification URL and device code to the console.
- After browser sign-in completes, the client receives the token and proceeds.
- Refresh tokens are persisted in the cache and used for automatic refresh.

## Sample Scenario

See [`samples/v1-features/src/scenarios/device_code_auth.rs`](../samples/v1-features/src/scenarios/device_code_auth.rs).
