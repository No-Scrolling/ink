---
title: "Barcodes and passes"
description: "Display codes and scan them natively."
---

Use `Barcode` from `@ink/barcode` to display codes, or `@ink/barcode/scan` to scan them. Displaying codes does not include camera code in your app.

See [Codes](/codes) for the 13 supported formats, examples and sizing. Invalid payloads reject.

## Permissions

| Permission | Required for |
| --- | --- |
| `camera` | Barcode scanning. |

Generating a code does not require camera access.

```ts
import { lightos } from "@ink/lightos";

const permission = await lightos.requestPermission("camera");
```

Use `lightos.getPermission("camera")` to check without a prompt. See [Request a permission](/permissions-guide) for the returned statuses.

## Scan a pass

Choose the accepted formats inside your scanner component:

```ts
import { useCodeScanner } from "@ink/barcode/scan";

const scanner = useCodeScanner({ formats: ["qr"] });
```

Pass `scanner` to `CameraPreview` from the same import. See [Camera](/camera#capture-a-photo) for the preview layout.

### Read the result

`scanner.state.value` contains `{ text, format, rawBytes }`. Select **Scan again** to restart.

QR scanning supports dark-on-light and inverted light-on-dark codes.

`rawBytes` contains a `Uint8Array` when the decoder supplies bytes, otherwise `null`. It may differ from the UTF-8 encoding of `text`.

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

`intervalMs` limits repeat results for the same code. It defaults to 1,000 milliseconds and accepts 100–60,000. Different codes arrive immediately.

There is no scan timeout. Scanning ends when the screen releases the camera.

## A pass library

Save pass details in [Store](/store) and original documents with [Files](/files). Keep display names separate from encoded values.

Rotating codes and authenticated tickets require a provider integration. Copied codes may expire. Keep the issuer’s readable reference when available.
