---
title: "Cryptography"
description: "Native cryptographic operations and secure randomness."
tag: "Design specification"
---

The host profile supplies `crypto.getRandomValues()` and `crypto.randomUUID()` for identifiers and nonces. `@ink/crypto` adds asynchronous native hashing and supported key operations for protocols that need them.

```ts
import { digest } from "@ink/crypto";

const input = new TextEncoder().encode("content to verify");
const hash = await digest("SHA-256", input);
```

`digest` returns bytes. Encoding those bytes as hex or base64 belongs to the calling protocol. Hashes verify integrity only against a trusted expected value; hashing a downloaded file does not establish who published it.

Large file digests accept a managed file reference and run natively with cancellation, avoiding a full JavaScript copy. The package reports its supported algorithms. Unsupported algorithms fail explicitly, and libraries must not silently substitute an incompatible or weaker implementation.

Native key handles keep private material out of ordinary snapshots and JSON. Persist a key ID, reopen it and handle invalidation. Exportability and hardware backing depend on the requested operation and device; a handle alone is not a hardware guarantee.

Use [Secure store](secure-store.md) for saved credentials and [Auth](auth.md) for PKCE. A JavaScript cryptography package can be bundled when it matches the host profile, but CPU-heavy operations may merit a native adapter.
