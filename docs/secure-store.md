---
title: "Secure store"
description: "Store small secrets behind opaque handles."
tag: "Planned"
---

`@ink/secure-store` saves credentials and small key material without returning restored secret text to app state.

## Store a secret

Create a slot with `secret()` and save a string from a user action.

```tsx
import { secret } from "@ink/secure-store";
import { Button, Text, match, state } from "ink";

const tokenInput = state("");
const apiToken = secret("api.token");

<Button onPress={() => apiToken.store(tokenInput.value)}>Save token</Button>

{match(apiToken, {
  loading: () => <Text>Checking saved token</Text>,
  empty: () => <Text>No token saved</Text>,
  ready: () => <Text>Token saved</Text>,
  saving: () => <Text>Saving token</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

`store()` accepts 1 to 4,096 UTF-8 bytes and replaces the existing value atomically. `remove()` invalidates the current handle and deletes the saved record. `retry()` repeats the failed operation.

Keys must match `[A-Za-z0-9][A-Za-z0-9._-]{0,127}`. An app can keep up to 64 secret slots.

## Use a secret for a request

A ready slot provides an opaque `StoredSecret`. Pass it to `bearer()` without reading the token:

```tsx
import { bearer, json } from "@ink/network";

type Profile = { name: string };

if (apiToken.status === "ready") {
  const profile = json<Profile>("https://api.example/profile", {
    authorization: bearer(apiToken.secret),
  });
}
```

`StoredSecret` has no string conversion or serialised form. You cannot put it in state, route data, files, or another store.

`@ink/auth` uses secure store for access, refresh, and identity tokens. `@ink/crypto` uses it for private signing keys. These packages do not expose the stored bytes to your app.

## Lifecycle and errors

Secret slots are app-scoped. Declarations with the same key share one status and handle. Secrets survive process death and app upgrades, but not app-data clearing or uninstall. Android backup and device transfer exclude them.

Removing a secret invalidates existing handles immediately. An authenticated request or signing operation that receives an invalidated handle fails without using the old value.

Errors distinguish a locked device, an invalidated encryption key, unavailable secure storage, quota limits, storage failures, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.

The package does not open a permission or biometric prompt. A `locked` error can be retried after the device is unlocked. A `key-invalidated` secret must be replaced or removed.
