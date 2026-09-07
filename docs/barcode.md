---
title: "Barcodes and passes"
description: "Display codes and scan them natively."
---

Use `Barcode` from `@ink/barcode` to display a code. It supports 13 formats and does not include camera code.

To scan codes, use `@ink/barcode/scan`.

QR scanning accepts dark-on-light and inverted light-on-dark codes. Inverted QR detection has been confirmed on the LP3.

```tsx
import { Screen, Text } from "ink";
import { Barcode } from "@ink/barcode";

export function Pass({ title, value }: { title: string; value: string }) {
  return (
    <Screen title={title}>
      <Barcode format="qr" value={value} size={280} />
      <Text>{value}</Text>
    </Screen>
  );
}
```

Generation happens natively with sharp modules, appropriate contrast and quiet zones. Preserve the source payload exactly; trimming, normalising case or converting a numeric-looking string can invalidate it. The package exposes supported formats and rejects invalid payloads rather than rendering a misleading code.

## Size a code

`size` sets the width. QR codes stay square; PDF417 follows its encoded proportions; linear codes use a compact height. QR codes have a two-module white border. The template’s Code generation page shows all 13 formats, including `Hello World!` as a QR code at size 240.

## Scan a pass

`useCodeScanner({ formats })` from `@ink/barcode/scan` manages a visible scanner attachment. Pass it to `CameraPreview` from the same entry point. A decoded result appears in `scanner.state.value` as `{ text, format, rawBytes }`; the preview offers Scan again, or call `scanner.open()` to restart. Permission queries and requests are available through the exported `camera` object. Use `continuous: true` to keep the preview active and receive results through `scanner.state.value`. `intervalMs` (default 1000, range 100–60000) limits repeated delivery of the same code; a different code is delivered immediately. Continuous scanning runs until the screen loses its camera attachment and has no scan timeout. `rawBytes` is a `Uint8Array` when the decoder supplies bytes, otherwise `null`; these are decoder bytes, not necessarily the UTF-8 encoding of `text`.

Scanning stays native. Release the camera on leaving the screen. A scan result is untrusted input: show a link before opening it and validate app-specific formats before storing records.

## A pass library

Store ID, title, format, payload, sort order and optional expiry in [Store](/store). Keep original documents with [Files and media](/files). Editing a display name must not change the encoded value.

Tickets with rotating codes or provider authentication need a provider integration. A screenshot or copied payload does not guarantee that the resulting pass remains valid. Keep a fallback human-readable reference where the issuer provides one.
