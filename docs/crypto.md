---
title: "Crypto"
description: "Hash, sign, verify, and generate secure tokens."
---

`@ink/crypto` provides fixed, portable cryptographic operations with stable encodings. Text input uses UTF-8.

## Hash a value

Use `sha256()` to create a content digest. The ready value is 64 lower-case hexadecimal characters.

```tsx
import { sha256 } from "@ink/crypto";
import { Text, match, state } from "ink";

const note = state("");
const digest = sha256(note.value);

{match(digest, {
  loading: () => <Text>Calculating digest</Text>,
  ready: ({ value }) => <Text>{value}</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

Inputs are limited to 1 MiB. SHA-256 is suitable for content identity and checks against a trusted digest. Do not use it to store passwords.

## Generate a token

Use `randomToken()` to generate a 256-bit value encoded as unpadded Base64url.

```tsx
import { randomToken } from "@ink/crypto";
import { Button, Text } from "ink";

const token = randomToken();

<Button onPress={() => token.generate()}>Generate token</Button>

{token.status === "ready" ? <Text>{token.value}</Text> : null}
```

Each `generate()` call replaces the previous value. Calling it while generation is active has no effect. Generated tokens belong to the declaring screen and are not persisted.

## Sign a message

Use `signingKey()` to create or restore an Ed25519 key and sign UTF-8 messages.

```tsx
import { signingKey } from "@ink/crypto";
import { Button, Text, state } from "ink";

const payload = state("");
const deviceKey = signingKey("device.identity");

<Button onPress={() => deviceKey.sign(payload.value)}>Sign payload</Button>

{deviceKey.status === "signed" ? (
  <Text>{deviceKey.signature}</Text>
) : null}
```

The first declaration creates a key when none exists. Its private key remains in `@ink/secure-store`. The controller exposes only the public key and signatures, encoded as unpadded Base64url.

Keys must match `[A-Za-z0-9][A-Za-z0-9._-]{0,127}`. Messages are limited to 1 MiB. Declarations with the same key share one app-wide signing key.

## Verify a signature

Use `verifyEd25519()` with the original message, signature, and public key.

```tsx
import { verifyEd25519 } from "@ink/crypto";
import { Text, match, state } from "ink";

const payload = state("");
const receivedSignature = state("");
const senderPublicKey = state("");
const verified = verifyEd25519({
  message: payload.value,
  signature: receivedSignature.value,
  publicKey: senderPublicKey.value,
});

{match(verified, {
  loading: () => <Text>Checking signature</Text>,
  ready: ({ value }) => <Text>{value ? "Valid" : "Invalid"}</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

A correctly encoded signature that does not match returns `false`. Malformed Base64url, wrong decoded lengths, and oversized messages return errors.

## Lifecycle and errors

Hash and verification resources are screen-scoped. Leaving the screen cancels active work and ignores late results.

Signing keys are app-scoped and survive process death and app upgrades. They do not survive app-data clearing or uninstall. Calls for one key are serialised, and only the latest completed signature is published.

Errors distinguish invalid encodings, oversized input, unavailable keys, secure-store failures, unavailable cryptography, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.
