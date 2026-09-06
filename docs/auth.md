---
title: "Accounts and sign-in"
description: "Sign in and keep a session without managing token refresh in screens."
tag: "Planned"
---

> **Not implemented yet.** This page defines the intended interface.

`@ink/auth` owns OAuth sign-in, secure token persistence, refresh and sign-out. Apps configure their provider; screens do not store tokens or implement refresh loops.

```ts
import { createOAuthClient } from "@ink/auth";

export const account = createOAuthClient({
  id: "music",
  clientId: "public-mobile-client-id",
  tokenEndpoint: "https://accounts.example.com/token",
  scopes: ["library.read"],
  deviceAuthorizationEndpoint: "https://accounts.example.com/device",
});
```

`id` is a stable, app-local session key. Recreating the client with the same configuration reconnects to that persisted session. Different IDs keep credentials separate. The first release does not provide an account-switching interface.

## Device-code sign-in

`account.startDeviceSignIn({ signal })` returns `verificationUri`, optional `verificationUriComplete`, `userCode`, `expiresAt` and `complete({ signal })`. Display the code or generate a QR code from the verification URL, then await completion. Ink owns polling intervals, slowdown responses, expiry and cancellation. Completion resolves after the session is persisted.

This supports the flow used by Index and Echo TV without requiring a browser on the phone. The provider must support device authorisation.

## Browser sign-in

Configure `authorizationEndpoint` and `redirectUri` alongside the token endpoint. `account.signIn({ signal })` presents a browser-backed in-app window using Android Custom Tabs and completes the session. Closing it returns to the same Ink screen. Ink owns PKCE, state validation, redirect handling and the activity round trip. Register the redirect with the provider and Android app.

Keep the flow visually close to LightOS using the browser's supported colour and toolbar customisation, while retaining its site identity and security UI. This is not an embedded WebView: OAuth uses an isolated browser context, as described in [OAuth for native apps](https://www.rfc-editor.org/rfc/rfc8252.html).

Read-only ADB inspection on 6 September 2026 found that the connected LP3's Chromium 133.0.6888.0 advertises Custom Tabs and colour customisation; see [device evidence](verification-custom-tabs.md). An actual browser sign-in and return flow still needs implementation and verification. Feature-detect support on each installation. If unavailable, report that browser sign-in is unavailable and offer device-code sign-in when configured. Do not silently launch the full browser or substitute a WebView.

A device-only configuration does not require browser settings. Calling a flow without its required configuration rejects clearly. A mobile app cannot keep a client secret private; any secret-based exchange belongs in a provider backend.

## Use the session

`account.getAccessToken({ signal })` returns a current token, refreshing when necessary. Overlapping calls share refresh work. Credential writes are coordinated across foreground and worker runtimes so an older refresh cannot overwrite a newer session or undo sign-out. Cancelling one caller does not cancel work still needed by another.

`useSnapshot(account)` uses Ink's standard `loading`, `ready` and `error` snapshot. Ready data contains a `status` of `signed-out` or `signed-in`, without tokens. Provider profile data belongs to the app. A provider request function obtains a token and uses ordinary `fetch` or a compatible JavaScript client.

A worker imports the same configuration and opens its session from [Secure store](secure-store.md). If interactive sign-in is required, it reports that outcome rather than opening UI.

## Sign out

`account.signOut()` cancels session-owned work and removes local credentials. It does not depend on remote revocation succeeding. Apps separately clear personalised caches, queued changes and downloads.

Unusual exchanges, remote revocation and service-specific behaviour belong in provider integrations. Ink does not introduce a provider-plugin registry or generic HTTP retry layer.
