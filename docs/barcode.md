---
title: "Barcode"
description: "Scan supported barcodes and generate barcode images."
tag: "Planned"
---

`@ink/barcode` presents a barcode-owned scanning interface and generates barcode images in process. Its scanner uses the Camera implementation internally, but callers do not coordinate the two modules.

## Scan a barcode

Create a scanner and pass it to `BarcodeScannerView`:

```tsx
import { BarcodeScannerView, barcodeScanner } from "@ink/barcode";
import { Screen, Text } from "ink";

const scanner = barcodeScanner({
  formats: ["qr", "ean-13", "code-128"],
  permissionPrompt: "Allow camera to scan a barcode",
  timeoutMs: 60_000,
});

<Screen title="Scan code">
  <BarcodeScannerView session={scanner} />
  {scanner.phase === "ready" ? <Text>{scanner.value.text}</Text> : null}
</Screen>
```

The standard view presents an explicit permission action, viewfinder, torch control, and timeout state. Creating a scanner never opens a permission prompt. For a custom layout, use `scanner.permission.request()` from a user action.

The first stable result stops analysis and returns `text`, optional raw `bytes`, `format`, `cornerPoints`, and `scannedAtMs`. Call `scanAgain()` to resume.

Supported scan formats are QR, Aztec, Data Matrix, PDF417, Codabar, Code 39, Code 93, Code 128, EAN-8, EAN-13, ITF, UPC-A, and UPC-E.

## Scan continuously

Pass `mode: "continuous"` for inventory or event workflows. Results are delivered through a bounded event queue:

```tsx
const scanner = barcodeScanner({
  formats: ["qr"],
  mode: "continuous",
  delivery: { capacity: 16, overflow: "error" },
});
```

Ink deduplicates the same stable value while it remains in view. The session reports dropped events only when `overflow: "drop-oldest"` is selected explicitly.

## Generate a barcode

`barcodeImage()` is a pure computed operation and does not create a loading resource:

```tsx
const ticket = barcodeImage({
  format: "qr",
  value: "https://example.com/ticket/42",
  width: 280,
  height: 280,
  correction: "medium",
});

{ticket.ok ? (
  <Image src={ticket.value.source} width={280} height={280} fit="contain" />
) : (
  <Text>{ticket.error.message}</Text>
)}
```

Generation supports QR, Aztec, Data Matrix, Code 128, and EAN-13. It links only the in-process generator artefact, not Camera.

## Lifecycle and errors

The scanner is a screen-owned session. Leaving the screen or backgrounding the app releases Camera. Only one Camera or Barcode session can be open.

Scanning errors distinguish permission, unavailable or busy Camera, unsupported formats, timeout, decoding, queue overflow, and unexpected failures. Generation errors distinguish invalid values, options, and unsupported formats. Every error provides `kind`, `message`, and `retryable`.
