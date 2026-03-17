# Client Credentials Auth

Client credentials auth is supported through Dataverse-style connection strings and through `AuthConfig::ClientCredentials`.

## Connection String

```text
AuthType=ClientSecret;Url=https://YOUR_ORG.crm.dynamics.com;ClientId=CLIENT_ID;ClientSecret=CLIENT_SECRET;TenantId=TENANT_ID;TokenCacheStorePath=C:\MyTokenCache\token-cache.txt
```

## Notes

- `ServiceClient` acquires the access token internally.
- Tokens are refreshed automatically before request execution when the token is close to expiry.
- `TokenCacheStorePath` can point to either a folder or a file.

## Sample Scenario

See [`samples/v1-features/src/scenarios/client_credentials_auth.rs`](../samples/v1-features/src/scenarios/client_credentials_auth.rs).
