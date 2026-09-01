---
title: "Barcode"
description: "Scan supported barcodes and generate barcode images."
---

`@ink/barcode` scans barcodes through a native camera view and generates barcode images for display. Scanning uses camera permission from `@ink/camera`; generation does not use the camera.

## Request camera permission

Call `cameraPermission()` from `@ink/camera` before opening a scanner.

```tsx
import { cameraPermission } from "@ink/camera";
import { Button, Text, match } from "ink";

const permission = cameraPermission();

{match(permission, {
  loading: () => <Text>Checking camera permission</Text>,
  ready: (result) => <Text>{result.value}</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}

<Button onPress={() => permission.request()}>Allow camera</Button>
```

Creating a scanner does not open a permission prompt. Generation requires no permission.

## Scan a barcode

`barcodeScanner()` scans QR codes by default. Pass its controller to `BarcodeScannerView`.

```tsx
import { BarcodeScannerView, barcodeScanner } from "@ink/barcode";
import { Button, Screen, Text } from "ink";

export default function Scan() {
  const scanner = barcodeScanner({
    formats: ["qr", "ean-13", "code-128"],
    timeoutMs: 60_000,
  });

  return (
    <Screen title="Scan code">
      <BarcodeScannerView controller={scanner} />
      {scanner.status === "ready" ? (
        <>
          <Text>{scanner.value.text}</Text>
          <Button onPress={() => scanner.scanAgain()}>Scan another</Button>
        </>
      ) : null}
    </Screen>
  );
}
```

The first stable result stops scanning and changes the controller to `ready`. Its value contains:

- `text`, the decoded string;
- `format`, the recognised format;
- `cornerPoints`, logical coordinates within the scanner view;
- `scannedAtMs`, a Unix timestamp in milliseconds.

Call `scanAgain()` to clear the result and resume. Call `setTorch("on")` or `setTorch("off")` while scanning to control the torch.

Scanner options are:

| Option | Values | Default |
| --- | --- | --- |
| `formats` | A literal list of supported formats | `["qr"]` |
| `facing` | `"back"`, `"front"` | `"back"` |
| `torch` | `"off"`, `"on"` | `"off"` |
| `timeoutMs` | 1,000 to 120,000 milliseconds | 60,000 milliseconds |

Supported scan formats are QR, Aztec, Data Matrix, PDF417, Codabar, Code 39, Code 93, Code 128, EAN-8, EAN-13, ITF, UPC-A, and UPC-E.

## Generate a barcode

`barcodeImage()` returns an opaque image source accepted by `Image.src`.

```tsx
import { barcodeImage } from "@ink/barcode";
import { Image, Text, match } from "ink";

const ticket = barcodeImage({
  format: "qr",
  value: "https://example.com/ticket/42",
  width: 280,
  height: 280,
  correction: "medium",
});

{match(ticket, {
  loading: () => <Text>Generating code</Text>,
  ready: (result) => (
    <Image
      src={result.value.source}
      width={280}
      height={280}
      fit="contain"
    />
  ),
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

Generation supports QR, Aztec, Data Matrix, Code 128, and EAN-13. The value must satisfy the selected format's character, length, and check-digit rules.

`correction` accepts `"low"`, `"medium"`, `"quartile"`, or `"high"` for formats with error correction. Omit it for Code 128 and EAN-13.

The ready value contains `source`, `format`, `width`, and `height`. The source cannot be read as a bitmap or filesystem path.

## Scanner states

| Status | Meaning |
| --- | --- |
| `idle` | The scanner view is not active. |
| `opening` | The camera is starting. |
| `scanning` | The view is analysing supported codes. |
| `ready` | A stable code was recognised. |
| `error` | Scanning failed. |

Leaving the screen, pressing the standard back button, or moving the app to the background releases the camera. Only one camera or barcode session can be open. A second session receives a `busy` error.

Generated images are screen-scoped and remain available while their resource is active.

## Errors

Errors provide `kind`, `message`, and `retryable`.

Scanning errors distinguish denied or blocked permission, unsupported formats, a busy or unavailable camera, timeout, decoding failure, and unexpected failures. Generation errors distinguish unsupported formats, invalid values or options, generation failure, and unexpected failures.
