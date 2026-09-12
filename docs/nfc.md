---
title: "NFC"
description: "Read supported tags through native sessions."
---

Use `@ink/nfc` to read tags, send raw commands or emulate a card. Tag writing is not supported.

## Read a tag

`nfc.read()` reads NFC Data Exchange Format (NDEF) records:

```ts
import { nfc } from "@ink/nfc";

const tag = await nfc.read({ signal, timeout: 30_000 });
const shortcut = decodeShortcut(tag.records);
await openShortcut(shortcut);
```

Supply a cancellation `signal`, a `decodeShortcut` validator and an `openShortcut` handler.

Reading waits for a tag while the app is visible. Handle cancellation, timeout, disabled NFC and unsupported tags separately.

### Validate tag data

Validate records before using them. Let the user review a tag’s URL before opening it. Tag IDs are not proof of identity.

### Session lifetime

Sessions close when leaving the screen or entering the background. Reading NDEF requires neither a raw connection nor card emulation.

## Exchange raw commands

Use `iso-dep` or `nfc-a` to exchange bytes using the tag’s protocol:

```ts
const connection = await nfc.connect({ technology: "iso-dep", signal });
try {
  const response = await connection.transceive(commandBytes, { signal, timeout: 5000 });
  // Decode the response using the tag's application protocol.
} finally {
  await connection.close();
}
```

Commands and responses are `Uint8Array` values. The connection reports its technology, serial number and maximum command length.

### Limits and cancellation

Only one raw connection or NDEF reader can run at a time. Commands run in order, with a timeout of up to ten seconds and a 64 KiB response limit.

The signal passed to `connect()` cancels discovery. Close the returned connection when leaving the screen. Cancelling an exchange or backgrounding the app also closes it.

## Card emulation

### Register static responses

```ts
await nfc.emulate({
  aids: ["F000000001"],
  responses: [{ command: "00A4040005F000000001", response: "9000" }],
  fallback: "6D00",
});
```

This example registers application identifier (AID) `F000000001`. The listed command returns `9000` (success); other commands return `6D00` (unsupported).

AIDs are hexadecimal values registered in Android’s `other` category. Static responses work without the app screen open. To remove the registration:

```ts
await nfc.stopEmulation();
```

The device must support host card emulation (HCE) and be unlocked. Payment and secure-element integration are not supported.

### Dynamic responses

After registration, handle commands while the app is visible. Supply your protocol’s `respondToCommand` function and a cancellation `signal`:

```ts
await nfc.handleApdu(async command => {
  return respondToCommand(command);
}, { signal });
```

Return response bytes including the status word.

`deadline` defaults to 500 milliseconds and accepts 50–2,000. Late responses are discarded; Ink uses the static response or fallback instead.

Readers may need a faster response, so avoid network requests in the handler.

Backgrounding or aborting the handler restores static responses without removing the AIDs.

### Emulation limits

| Resource | Maximum |
| --- | --- |
| Static rules | 256 |
| Command or response | 4,096 bytes |
| Registered AIDs | 32 |
| JavaScript handlers | 1 |
