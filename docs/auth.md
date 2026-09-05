---
title: "Accounts and sign-in"
description: "Browser OAuth, device codes and reusable provider sessions."
tag: "Planned"
---

> **Planned.** This package is not implemented. The APIs below describe the proposed design.

`@ink/auth` handles OAuth mechanics. A provider module supplies endpoints, scopes, response decoding and account identity. Keep provider details outside screens.

```ts
import { createOAuthClient } from "@ink/auth";

export const account = createOAuthClient({
  id: "example-music",
  clientId: "public-mobile-client-id",
  authorizationEndpoint: "https://accounts.example.com/authorize",
  tokenEndpoint: "https://accounts.example.com/token",
  redirectUri: "com.example.music:/oauth",
  scopes: ["library.read"],
});
```

Register the redirect with both the provider and the app manifest. `account.signIn({ signal })` launches the system browser with PKCE and validates the returned state and redirect. Browser login remains owned through its external activity round trip. A mobile bundle cannot keep a client secret private; providers requiring one need a backend.

## Use the session

`account.getAccessToken({ signal })` returns a current token, refreshing when needed. Simultaneous calls share one refresh attempt. The account module persists credentials in [Secure store](secure-store.md) and exposes a readable account snapshot for `useSnapshot`.

A provider's `getPlaylists()` function obtains a token and makes its own request. This works with ordinary npm clients that accept a token or transport. Do not put tokens in route parameters or UI snapshots. On an authentication failure, reconcile with the provider before retrying; repeated refresh loops waste power and hide revoked access.

## Device codes

`account.startDeviceSignIn({ signal })` returns the provider's verification URL, user code, expiry and a `complete({ signal })` operation. Show the code and URL on the phone. Completion respects the provider's polling interval, slowdown responses, cancellation and expiry.

A device-code flow is useful for a feed reader or media provider when browser redirects are unavailable. The provider must support it; Ink cannot add it to an arbitrary service.

## Account lifecycle

`signOut()` cancels owned work, removes local credentials and clears the session. Remote revocation is provider-dependent and may fail offline. A domain module separately clears personalised caches, queued mutations and downloads according to its retention policy.

Multiple accounts have separate IDs, caches and refresh state. Background workers reopen an account by ID from secure storage; they cannot reuse a foreground JavaScript object. If reauthentication is needed, defer work and let the foreground explain it.
