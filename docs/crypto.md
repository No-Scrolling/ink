---
title: "Encryption"
description: "Encrypt and decrypt text with a shared key."
---

`@ink/crypto` encrypts text and detects tampering using Android's AES-256-GCM implementation. It is only bundled with apps that use crypto or LightOS push.

## Encrypt and decrypt

```ts
import { createCipher } from "@ink/crypto";

const cipher = createCipher(encryptionKey);
const encrypted = await cipher.encrypt("Hello");
const original = await cipher.decrypt(encrypted);
```

Both methods return strings. Decryption rejects incorrect keys, damaged data and unsupported formats. Ink generates a fresh nonce for each encryption.

## Keys

The sender and receiver need the same base64-encoded, random 32-byte key:

```sh
openssl rand -base64 32
```

Use [Secure storage](/secure-store) to save a key on the device. A key included through an environment variable remains readable from the APK.

## Structured data

Encode objects as JSON before encryption and validate the decoded data before using it:

```ts
const encrypted = await cipher.encrypt(JSON.stringify({ text: "Hello" }));
const data: unknown = JSON.parse(await cipher.decrypt(encrypted));
```

Encryption does not expire data or prevent repeated deliveries. Handle these in your app, or use the expiry and duplicate detection in [push notifications](/light-sdk#send-a-notification).

## Server format

The output is `ink1.` followed by standard padded base64. The decoded bytes contain:

| Part | Size |
| --- | --- |
| Random nonce | 12 bytes |
| AES-256-GCM ciphertext of UTF-8 text | Variable |
| Authentication tag | 16 bytes |

Use the UTF-8 bytes of `ink1.` as GCM additional authenticated data. No separate signing key is needed.

Your server can use its standard AES-GCM implementation.
