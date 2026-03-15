# Token Cache

The client can persist token state to disk so sessions can survive process restarts.

## Connection String Parameter

```text
TokenCacheStorePath=C:\MyTokenCache
```

or

```text
TokenCacheStorePath=C:\MyTokenCache\token-cache.txt
```

## Notes

- If the value is a folder, the client writes `token_cache.txt` into that folder.
- If the value is a file path, the client writes directly to that file.
- If omitted, the client falls back to `data_local_dir()/powerplatform-dataverse-client/<guid>/token_cache.txt`.
- The cache currently stores JSON containing the access token and refresh token.

## Related Scenarios

- [`device_code_auth.rs`](../samples/v1-features/src/scenarios/device_code_auth.rs)
- [`client_credentials_auth.rs`](../samples/v1-features/src/scenarios/client_credentials_auth.rs)
- [`refresh_demo.rs`](../samples/v1-features/src/scenarios/refresh_demo.rs)
