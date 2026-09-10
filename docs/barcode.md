---
title: "Barcodes and passes"
description: "Display codes and scan them natively."
---

Use `Barcode` from `@ink/barcode` to display a code. It supports 13 formats and does not include camera code.

To scan codes, use `@ink/barcode/scan`.

QR scanning accepts dark-on-light and inverted light-on-dark codes. Inverted QR detection has been confirmed on the LP3.

See [Codes](/codes) for generation examples, formats and sizing. The package supports 13 formats and rejects invalid payloads.

## Scan a pass

`useCodeScanner({ formats })` from `@ink/barcode/scan` manages a visible scanner attachment. Pass it to `CameraPreview` from the same entry point. A decoded result appears in `scanner.state.value` as `{ text, format, rawBytes }`; the preview offers Scan again, or call `scanner.open()` to restart. Permission queries and requests are available through the exported `camera` object. Use `continuous: true` to keep the preview active and receive results through `scanner.state.value`. `intervalMs` (default 1000, range 100–60000) limits repeated delivery of the same code; a different code is delivered immediately. Continuous scanning runs until the screen loses its camera attachment and has no scan timeout. `rawBytes` is a `Uint8Array` when the decoder supplies bytes, otherwise `null`; these are decoder bytes, not necessarily the UTF-8 encoding of `text`.

Scanning stays native. Release the camera on leaving the screen. A scan result is untrusted input: show a link before opening it and validate app-specific formats before storing records.

## A pass library

Store ID, title, format, payload, sort order and optional expiry in [Store](/store). Keep original documents with [Files and media](/files). Editing a display name must not change the encoded value.

Tickets with rotating codes or provider authentication need a provider integration. A screenshot or copied payload does not guarantee that the resulting pass remains valid. Keep a fallback human-readable reference where the issuer provides one.
