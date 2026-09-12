---
title: "Barcodes and passes"
description: "Display codes and scan them natively."
---

Use `Barcode` from `@ink/barcode` to display codes, or `@ink/barcode/scan` to scan them. Displaying codes does not include camera code in your app.

See [Codes](/codes) for the 13 supported formats, examples and sizing. Invalid payloads reject.

## Permissions

Scanning requires camera access. Generating a code does not.

```ts
import { camera } from "@ink/barcode/scan";

const permission = await camera.requestPermission();
```

Use `camera.getPermission()` to check access without a prompt. Both methods return `granted`, `denied` or `blocked`. See [Request a permission](/permissions-guide) for handling each result.

## Scan a pass

Inside your scanner component, choose which formats to accept:

```ts
import { useCodeScanner } from "@ink/barcode/scan";

const scanner = useCodeScanner({ formats: ["qr"] });
```

Pass `scanner` as the controller to `CameraPreview` from the same import. See [Camera](/camera#capture-a-photo) for the preview layout. The result appears in `scanner.state.value` as `{ text, format, rawBytes }`. Select **Scan again** to restart.

QR scanning supports dark-on-light and inverted light-on-dark codes. `rawBytes` contains a `Uint8Array` when the decoder supplies bytes, otherwise `null`. These bytes may differ from the UTF-8 encoding of `text`.

Scanning runs natively. Release the camera when leaving the screen. Validate scanned values before storing them, and let the user review links before opening them.

### Continuous scanning

To keep scanning, replace the hook configuration with:

```ts
const scanner = useCodeScanner({
  formats: ["qr"],
  continuous: true,
  intervalMs: 1_000,
});
```

Results appear in `scanner.state.value`.

`intervalMs` limits repeat results for the same code. It defaults to 1,000 milliseconds and accepts 100–60,000. Different codes arrive immediately. There is no scan timeout; scanning ends when the screen releases the camera.

## A pass library

Save pass details in [Store](/store) and original documents with [Files](/files). Keep display names separate from encoded values.

Rotating codes and authenticated tickets require a provider integration. Copied codes may expire. Keep the issuer’s readable reference when available.
