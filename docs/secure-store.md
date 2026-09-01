---
title: "Secure store"
description: "Store small secrets behind opaque references."
tag: "Planned"
---

`@ink/secure-store` saves credentials and small secret material without returning restored text to app state.

## Store a secret

Create a slot with `secret()` and save from a user action:

```tsx
import { secret } from "@ink/secure-store";
import { Button, Text, state } from "ink";

const tokenInput = state("");
const apiToken = secret("api.token");

<Button onPress={() => apiToken.store(tokenInput.value)}>Save token</Button>

{apiToken.phase === "ready" ? <Text>Token saved</Text> : null}
{apiToken.phase === "empty" ? <Text>No token saved</Text> : null}
```

The slot phases are `loading`, `empty`, `ready`, and `error`. Its `action` reports `idle`, `running`, `success`, or `error` for `store()` and `remove()`.

`store()` accepts 1 to 4,096 UTF-8 bytes and replaces the existing value atomically. `remove()` invalidates its current generation immediately, then deletes the saved record.

Package-created keys are automatically namespaced. App keys must match `[A-Za-z0-9][A-Za-z0-9._-]{0,127}`. An app can keep up to 64 secret slots.

## Use a secret

Every slot exposes a stable opaque reference, even while it is loading or empty. Pass the slot to an operation that understands its purpose:

```tsx
const apiToken = secret("api.token");

const profile = json<Profile>("https://api.example/profile", {
  authorization: bearer(apiToken),
});
```

Network waits while the slot restores, reloads when its generation changes, and returns `authentication-required` while it is empty. This keeps resource declarations stable and hides restoration ordering from callers.

The reference has no string conversion or serialised form. It cannot enter state, route data, Files, or Store. Access is granted only to the operation receiving it, and its readable value is omitted from diagnostics.

Auth uses secure storage for provider credentials. Crypto signing identities use Android Keystore directly and do not store exportable private key bytes in this module.

## Lifecycle and errors

Secret slots are application-scoped and survive process death and upgrades, but not app-data clearing or uninstall. Android backup and device transfer exclude them.

Existing references observe generation invalidation immediately. An operation that already acquired a short-lived lease can finish; later operations cannot use the removed generation.

Errors distinguish a locked device, invalidated encryption keys, unavailable secure storage, quota limits, storage failures, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.

The package does not open a permission or biometric prompt. A `locked` error can be retried after the device is unlocked. A `key-invalidated` secret must be replaced or removed.
