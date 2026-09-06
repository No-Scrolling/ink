---
title: "NFC"
description: "Read supported tags through native sessions."
---

Use `@ink/nfc` to read NDEF tags, exchange raw ISO-DEP or NfcA commands, or emulate an app-defined card. Tag writing is not supported.

```ts
import { nfc } from "@ink/nfc";

const tag = await nfc.read({ signal, timeout: 30_000 });
const shortcut = decodeShortcut(tag.records);
await openShortcut(shortcut);
```

This fragment assumes a cancellation signal and app-specific validation/navigation functions. Reading waits for a tag while the app is foregrounded. Cancellation, timeout, disabled NFC and unsupported tags are distinct outcomes.

Inspect record types and validate payloads before acting. A tag URL does not authorise opening an arbitrary destination automatically. Tag identifiers are useful lookup hints, not proof of identity.

Sessions release on leaving the screen or backgrounding. Reading NDEF does not require a raw connection or card emulation.

## Raw connections

```ts
const connection = await nfc.connect({ technology: "iso-dep", signal });
try {
  const response = await connection.transceive(commandBytes, { signal, timeout: 5000 });
  // Decode the response using the tag's application protocol.
} finally {
  await connection.close();
}
```

`iso-dep` and `nfc-a` expose native exchanges as `Uint8Array` values. The result identifies the technology, serial number and maximum command length. Only one raw connection or NDEF reader can own the radio at a time. Exchanges are serial, with a timeout of up to ten seconds and a 64 KiB response limit. Cancelling a pending operation or backgrounding the app closes its connection. The connection signal covers discovery; close the returned connection in your screen's cleanup as well.

Decode raw responses using the tag’s protocol. Support for a technology does not cover every application that uses it.

## Card emulation

```ts
await nfc.emulate({
  aids: ["F000000001"],
  responses: [{ command: "00A4040005F000000001", response: "9000" }],
  fallback: "6D00",
});
```

The template’s Enable Demo Card action uses this example: selecting the app-defined AID `F000000001` returns `9000` (success), and other commands return `6D00` (unsupported). Disable Demo Card removes the registration. It does not write a tag.

Register explicit hexadecimal AIDs in Android's `other` category. Exact command/response rules execute natively, including when the app UI is absent. Registration and responses persist until `await nfc.stopEmulation()`. This is host card emulation for an app protocol, without payment or secure-element integration; the device must support HCE and be unlocked.

For dynamic foreground responses, run `nfc.handleApdu(async command => responseBytes, { signal })` after registration. It receives command bytes and returns response bytes, including the status word. The native service waits up to `deadline` milliseconds (default 500, range 50–2000), then uses the matching static response or fallback. Late responses are discarded. A reader may impose a shorter deadline. Backgrounding or aborting the handler restores native fallback behaviour; it does not remove the registered AIDs. A JavaScript handler must finish promptly and should avoid network requests.

Rules are limited to 256 entries; commands and responses to 4096 bytes; registrations to 32 AIDs. One JavaScript handler can run at a time.
