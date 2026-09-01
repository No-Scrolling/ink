---
title: "Auth"
description: "Sign in through a browser or device-code flow and use an opaque authorisation."
tag: "Planned"
---

`@ink/auth` runs OAuth and OpenID Connect flows for public native clients. It restores and refreshes sessions without exposing access or refresh tokens to app code.

## Create a browser session

Declare one application-scoped session with the provider's HTTPS issuer, public client ID, and scopes:

```tsx
import { oauthSession } from "@ink/auth";
import { Button, Text } from "ink";

const account = oauthSession({
  key: "primary",
  issuer: "https://accounts.example.com",
  clientId: "ink-mobile",
  scopes: ["openid", "profile", "offline_access"],
  flow: "browser",
});

{account.phase === "signed-out" ? (
  <Button onPress={() => account.signIn()}>Sign in</Button>
) : null}
{account.phase === "signing-in" ? <Text>Complete sign-in</Text> : null}
{account.phase === "signed-in" ? <Text>Signed in</Text> : null}
```

The public phases are `restoring`, `signed-out`, `signing-in`, `signed-in`, `signing-out`, and `error`. Browser opening, provider authorisation, code exchange, validation, storage, and refresh remain implementation details inside `signing-in`.

The browser flow uses an Android Custom Tab or system browser and authorisation code with PKCE S256. Ink validates discovery metadata, redirects, state, nonce, code exchange, and token metadata.

## Use device-code sign-in

Use a device-code flow for providers designed around a code entered on another device:

```tsx
const account = oauthSession({
  key: "television",
  clientId: "ink-tv",
  scopes: ["playback"],
  flow: {
    kind: "device-code",
    deviceAuthorizationEndpoint: "https://accounts.example.com/device/code",
    tokenEndpoint: "https://accounts.example.com/token",
  },
});
```

While signing in, `account.prompt` contains the user code, verification URL, and expiry when the provider supplies them. Ink owns polling intervals, slow-down responses, expiry, cancellation, and token validation.

Provider packages should hide endpoint and scope configuration behind a domain interface when the same setup would otherwise be repeated by every app.

## Make an authenticated request

Every session exposes a stable `authorization` reference:

```tsx
const profile = json<Profile>("https://api.example/profile", {
  authorization: account.authorization,
});
```

The reference remains stable while the session restores or signs out. Network waits for restoration, refreshes expiring access, combines concurrent refreshes, and reloads consumers after the authorisation generation changes.

The signed-in state can expose validated `subject` and `expiresAtMs`. Fetch profile fields through a typed provider resource instead of reading token claims.

## Sign out and revoke

`signOut()` invalidates the local authorisation immediately and removes its stored credentials. Pass `{ revoke: true }` when the provider declares a compatible revocation endpoint and the app should attempt remote revocation.

Remote revocation is best effort and does not guarantee logout on another device. If secure deletion fails, `retry()` repeats it while the old authorisation remains invalid.

## Lifecycle, permissions, and errors

Sessions are application-scoped and survive navigation. Package-created keys are namespaced to the package. App keys with different provider options create independent accounts.

Refresh happens only when a consumer needs a valid authorisation. Auth does not keep the app awake. Background work can use an authorisation only when the session and provider declare it background-safe.

The package requests no Android runtime permission. Browser sign-in uses a package-scoped redirect activity and never accepts a client secret.

Errors distinguish unavailable browsers, invalid provider metadata, invalid redirects or responses, failed exchanges or refreshes, expired device codes, required interaction, secure-store failures, network failures, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.
