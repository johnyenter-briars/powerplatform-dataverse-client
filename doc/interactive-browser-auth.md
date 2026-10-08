# Interactive browser authentication

You can use the Microsoft Dataverse public client ID `51f81489-12ee-4a9e-aaae-a2591f45987d` with no new app registration. [Microsoft's Dataverse OAuth example](https://learn.microsoft.com/en-us/power-apps/developer/data-platform/authenticate-oauth) uses that ID with the `http://localhost` redirect for interactive sign-in. The client opens the default browser, listens on a temporary loopback port, and uses PKCE; no client secret is needed.

```rust
use powerplatform_dataverse_client::{LogLevel, auth::config::AuthConfig};
use powerplatform_dataverse_client::dataverse::serviceclient::ServiceClient;

let client = ServiceClient::new_with_auth(
    AuthConfig::InteractiveBrowser {
        client_id: "51f81489-12ee-4a9e-aaae-a2591f45987d".into(),
        dataverse_url: "https://YOUR_ORG.crm.dynamics.com".into(),
        tenant_id: "organizations".into(),
        token_cache_store_path: Some("./auth/browser-token.json".into()),
    },
    LogLevel::Information,
).await?;
```

For connection strings, use `AuthType=InteractiveBrowser;Url=https://YOUR_ORG.crm.dynamics.com;AppId=51f81489-12ee-4a9e-aaae-a2591f45987d;TenantId=organizations`. `AuthType=OAuth;AuthenticationFlow=InteractiveBrowser` also selects this flow. Existing `AuthType=OAuth` strings continue to use device code. If your organization requires a client ID you control, register a public client application with the **Mobile and desktop applications** redirect URI `http://localhost` and delegated Dataverse `user_impersonation` permission, then substitute its app ID.

The local callback times out after five minutes. When no cache path is supplied, the cache lives under the user's local app data directory in a stable directory for that tenant, app, and environment. Access and refresh tokens use the crate's JSON token cache; unlike the encrypted MSAL cache in FlowRunFinderV2, this file is **not encrypted**. Store it in a user-private location and do not commit it. This cache is not compatible with MSAL cache files.

The client checks its token cache before opening a browser. It reuses a valid access token or refreshes it with the cached refresh token. When no usable cached credentials remain, the default browser opens an account picker instead of signing in silently with that browser's current account. You can choose another account there or copy the sign-in URL from the browser into a different profile. Use a separate `TokenCacheStorePath` for each account or environment you want to keep distinct.
