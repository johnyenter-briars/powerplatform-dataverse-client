# Token Refresh

`ServiceClient` refreshes access tokens automatically before making requests when the current token is close to expiry.

## Notes

- Client-credentials auth refreshes by requesting a new access token.
- Device-code auth refreshes by using the cached refresh token.
- The refresh threshold is currently five minutes before expiry.
- Refresh state is stored in the token cache used by the client.

## Sample Scenario

See [`samples/v1-features/src/scenarios/refresh_demo.rs`](../samples/v1-features/src/scenarios/refresh_demo.rs).
