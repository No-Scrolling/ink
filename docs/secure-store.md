---
title: "Secure store"
description: "Store credentials with Android Keystore protection."
---

Use `@ink/secure-store` for small secret strings, such as tokens and credentials. Ink encrypts them with keys protected by Android Keystore.

For standard OAuth sign-in, use [Auth](/auth), which stores and refreshes tokens for you.

## Save a secret

```ts
import { secureStore } from "@ink/secure-store";

await secureStore.set("account.work.refresh-token", refreshToken);
```

## Read a secret

```ts
const savedToken = await secureStore.get("account.work.refresh-token");
```

`get()` returns a string, or `null` if the key is missing. Storage failures and invalidated encryption keys reject; a failed read does not mean the user is signed out.

## Remove a secret

Remove account credentials on sign-out:

```ts
await secureStore.remove("account.work.refresh-token");
```

## Protect credentials

All code in your app can read these values. Keep tokens out of component state, routes, analytics and logs.

Use separate keys for each account. Writes can fail, so handle rejected promises when saving or removing credentials.

### Backups and recovery

Encrypted values are excluded from portable backups. Invalidated encryption keys or a device transfer can require signing in again. Custom backup and recovery are not supported.

Store public account details separately so you can still display the account name after a session expires.
