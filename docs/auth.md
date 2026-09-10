---
title: "Accounts and sign-in"
description: "Sign in with OAuth and manage account sessions."
---

`@ink/auth` handles OAuth sign-in, token storage, refresh and sign-out. Configure it with your provider's settings.


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

Set the same custom redirect in `ink.toml`:

```toml
[auth]
redirect_uri = "ink-template://oauth/callback"
```

Endpoints must use HTTPS. Debug builds also permit HTTP on `localhost`, `127.0.0.1` and the Android emulator host `10.0.2.2` for local OAuth fixtures. Release builds reject these HTTP endpoints.

Custom Tabs uses a separate browser context and keeps the site identity and security controls visible. See [OAuth for native apps](https://www.rfc-editor.org/rfc/rfc8252.html).

Ink finds a compatible Custom Tabs browser on the device. If none is available, sign-in reports an unavailable error. You can offer device-code sign-in if your provider supports it. Ink does not fall back to a full browser or WebView.

A device-only configuration does not require browser settings. Calling a flow without its required configuration rejects clearly. A mobile app cannot keep a client secret private; any secret-based exchange belongs in a provider backend.

## Use the session

`account.getAccessToken({ signal })` returns a current token, refreshing when necessary. Overlapping calls share refresh work. Credential writes are coordinated across foreground and worker runtimes so an older refresh cannot overwrite a newer session or undo sign-out. Cancelling one caller does not cancel work still needed by another.

`useSnapshot(account)` uses Ink's standard `loading`, `ready` and `error` snapshot. Ready data contains a `status` of `signed-out` or `signed-in`, without tokens. Provider profile data belongs to the app. A provider request function obtains a token and uses ordinary `fetch` or a compatible JavaScript client.

A worker imports the same configuration and opens its session from [Secure store](/secure-store). If interactive sign-in is required, it reports that outcome rather than opening UI.

Android coordinates session access within the app process, including worker runtimes. Separate Android processes are not supported for Auth. Refresh completes before a waiting sign-out removes the credentials, and pending sign-in exchanges carry a generation that prevents them restoring a signed-out session.

## Sign out

`account.signOut()` cancels session-owned work and removes local credentials. It does not depend on remote revocation succeeding. Apps separately clear personalised caches, queued changes and downloads.

Handle provider-specific exchanges and remote token revocation in your app’s provider integration.
