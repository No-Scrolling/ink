---
title: "Secure store"
description: "Persist credentials using Android-backed protection."
---

`@ink/secure-store` persists small secret strings with native encryption and Android Keystore-backed key protection. Use it for refresh tokens, provider credentials and account secrets.

For standard OAuth sign-in, use [Auth](auth.md), which stores and refreshes tokens for you.

```ts
import { secureStore } from "@ink/secure-store";

await secureStore.set("account.work.refresh-token", refreshToken);
const savedToken = await secureStore.get("account.work.refresh-token");
await secureStore.remove("account.work.refresh-token");
```

`get()` returns `string | null`. Operations reject on unavailable storage, invalidated keys or write failures. A failed read is different from a missing value; do not quietly sign a user out on every storage error.

Secrets can be read by JavaScript when a provider library needs them. Encryption protects persisted data; it does not isolate a secret from dependencies running in the same app. Avoid placing tokens in component state, routes, analytics or logs.

Namespace values by account and delete them on sign-out. Native key invalidation or device transfer can require reauthentication. Encrypted values are excluded from portable backups; custom backup and recovery schemes are outside this scope. Store public account metadata separately so an expired session can still show the account name.

See the [verification record](verification-auth-secure-store-2026-09-06.md) for storage checks.
