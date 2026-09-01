---
title: "Crypto"
description: "Hash, verify, generate secure tokens, and use non-exportable signing identities."
tag: "Planned"
---

`@ink/crypto` provides a small set of fixed, portable cryptographic operations with stable encodings. Text input uses UTF-8.

## Hash a value

`sha256()` is a pure computed operation. It does not start a screen resource:

```tsx
import { sha256 } from "@ink/crypto";
import { Text, state } from "ink";

const note = state("");
const digest = sha256(note.value);

{digest.ok ? <Text>{digest.value}</Text> : <Text>{digest.error.message}</Text>}
```

The value is 64 lower-case hexadecimal characters. Inputs are limited to 1 MiB. SHA-256 is suitable for content identity and checks against a trusted digest. Do not use it to store passwords.

## Generate a token

Use `randomToken()` for explicit secure randomness:

```tsx
const token = randomToken({ bytes: 32, encoding: "base64url" });

<Button onPress={() => token.run()}>Generate token</Button>
```

The action uses `idle`, `running`, `success`, and `error`. Generated values belong to the declaring screen and are not persisted automatically.

## Create a signing identity

Use `signingIdentity()` to create or restore a purpose-bound, non-exportable Ed25519 identity in Android Keystore:

```tsx
const device = signingIdentity("device.identity", {
  purpose: "api-request-signing",
});

<Button onPress={() => device.sign(payload.value)}>Sign payload</Button>
```

The ready identity exposes its public key and a signing action. Private key bytes never enter Secure store, app state, diagnostics, or `app.ink`.

`rotate()` creates a new generation and invalidates future use of the old identity. `remove()` invalidates and deletes every retained generation. Pass an explicit retention policy when a protocol needs a previous public key during rotation.

Package-created keys are namespaced to the package. Intentional sharing requires an exported typed identity key.

## Verify a signature

`verifyEd25519()` is a pure computed operation:

```tsx
const verified = verifyEd25519({
  message: payload.value,
  signature: receivedSignature.value,
  publicKey: senderPublicKey.value,
});

{verified.ok ? <Text>{verified.value ? "Valid" : "Invalid"}</Text> : null}
```

A correctly encoded signature that does not match returns `false`. Malformed Base64url, wrong decoded lengths, and oversized messages return a typed result error.

## Lifecycle and errors

Pure operations recompute when their inputs change and have no cancellation or platform lifecycle. Signing identities are application-scoped and survive process death and upgrades, but not app-data clearing or uninstall. Calls for one identity are serialised.

Errors distinguish invalid encodings, oversized inputs, unavailable or invalidated identities, Keystore failures, unavailable cryptography, and unexpected failures. Action and session errors provide `kind`, `message`, `retryable`, and `operation`.
