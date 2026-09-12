---
title: "Secure store"
description: "Store credentials with Android Keystore protection."
---

Use `@ink/secure-store` for small secret strings, such as tokens and credentials. Ink encrypts them with keys protected by Android Keystore.

For standard OAuth sign-in, use [Auth](/auth), which stores and refreshes tokens for you.

```ts
import { secureStore } from "@ink/secure-store";

await secureStore.set("account.work.refresh-token", refreshToken);
```

Read the saved token:

```ts
const savedToken = await secureStore.get("account.work.refresh-token");
```

`get()` returns a string, or `null` if the key is missing. Unavailable storage, invalidated encryption keys and write failures reject. A failed read does not mean the user is signed out.

Remove it on sign-out:

```ts
await secureStore.remove("account.work.refresh-token");
```

## Protect credentials

JavaScript and dependencies in the same app can read these values. Encryption protects saved data; it does not isolate secrets from your app’s code. Keep tokens out of component state, routes, analytics and logs.

Use separate keys for each account and remove them on sign-out. Invalidated encryption keys or a device transfer can require signing in again. Encrypted values are excluded from portable backups; custom backup and recovery are not supported.

Store public account details separately so you can still display the account name after a session expires.
