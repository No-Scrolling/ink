---
title: "Barcodes and passes"
description: "Display codes and scan them natively."
tag: "Design specification"
---

`@ink/barcode` separates generation from scanning so a saved-pass viewer need not include camera code. Register the corresponding native entry points with `ink add`.

```tsx
import { Screen, Text } from "ink";
import { Barcode } from "@ink/barcode/generate";

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

## Scan a pass

`useScanner({ formats })` manages a visible scanner attachment. `ScannerPreview` displays it. The controller exposes permission/readiness state and decoded results containing format and payload. A single-shot scan pauses after a result; `resume()` explicitly starts the next scan. Continuous mode deduplicates repeated frames but is not a durable event log.

Scanning stays native. Release the camera on leaving the screen. A scan result is untrusted input: show a link before opening it and validate app-specific formats before storing records.

## A pass library

Store ID, title, format, payload, sort order and optional expiry in [Records](records.md). Keep originals in [Files](files.md) if a document contains information beyond the code. Editing a display name must not change the encoded value.

Tickets with rotating codes or provider authentication need a provider integration. A screenshot or copied payload does not guarantee that the resulting pass remains valid. Keep a fallback human-readable reference where the issuer provides one.
