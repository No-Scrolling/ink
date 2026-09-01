---
title: "Auth"
description: "Sign in through a browser and use an authenticated session."
tag: "Planned"
---

`@ink/auth` runs OAuth and OpenID Connect sign-in for public native clients. It restores and refreshes sessions without exposing access or refresh tokens to app code.

## Create a session

Declare one app-wide session with your provider's HTTPS issuer, public client ID, and scopes.

```tsx
import { oauthSession } from "@ink/auth";
import { Button, Text, match } from "ink";

const account = oauthSession({
  key: "primary",
  issuer: "https://accounts.example.com",
  clientId: "ink-mobile",
  scopes: ["openid", "profile", "offline_access"],
});

{match(account, {
  restoring: () => <Text>Restoring account</Text>,
  "signed-out": () => <Button onPress={() => account.signIn()}>Sign in</Button>,
  authorising: () => <Text>Complete sign-in in your browser</Text>,
  exchanging: () => <Text>Completing sign-in</Text>,
  "signed-in": () => <Text>Signed in</Text>,
  "signing-out": () => <Text>Signing out</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

Keys must match `[A-Za-z0-9][A-Za-z0-9._-]{0,127}`. Declarations with the same key share one session and must use identical options.

The issuer must provide valid discovery metadata. Ink derives the app redirect and lists it in `ink info`; register that exact value with the provider.

## Sign in

Call `signIn()` from a user action. Ink opens an Android Custom Tab or the system browser and uses the authorisation-code flow with PKCE S256.

Ink validates the provider, redirect, state, nonce, code exchange, and token metadata before the session becomes `signed-in`. Closing the browser or denying consent returns the session to `signed-out`.

Only one browser sign-in can be active in the app. Calling `signIn()` again while a flow is active has no effect.

## Make an authenticated request

Pass the session's opaque authorisation to `@ink/network`:

```tsx
import { json } from "@ink/network";

type Profile = { name: string };

if (account.status === "signed-in") {
  const profile = json<Profile>("https://api.example/profile", {
    authorization: account.value.authorization,
  });
}
```

`authorization` identifies the session but contains no readable token. Ink refreshes an expiring access token before the request starts and combines concurrent refreshes into one operation.

The signed-in value also provides `subject` when the provider supplies a validated OpenID Connect subject, plus `expiresAtMs` when the access-token expiry is known. Fetch profile fields through a typed network resource instead of reading token claims.

## Sign out

Call `signOut()` to invalidate the local authorisation immediately and remove the session from secure storage.

```tsx
<Button onPress={() => account.signOut()}>Sign out</Button>
```

Local sign-out does not revoke sessions on other devices or guarantee provider-wide logout. If secure deletion fails, `retry()` repeats it while the old authorisation remains invalid.

## Lifecycle, permissions, and errors

Sessions are app-scoped and survive navigation. Tokens are stored through `@ink/secure-store` and survive process death and app upgrades. Refresh happens only when a consumer needs a valid access token; auth does not keep the app awake.

If refresh requires user interaction, the session returns an `interaction-required` error. Call `signIn()` from a user action to continue.

The package requests no Android runtime permission. Sign-in uses the system browser and a package-scoped redirect activity. It never embeds a web view or accepts a client secret.

Errors distinguish an unavailable browser, invalid provider metadata, invalid redirects or responses, failed exchanges or refreshes, required user interaction, secure-store failures, network failures, unavailable auth state, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.
