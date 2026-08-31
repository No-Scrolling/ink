---
title: "Camera"
description: "Request camera access, capture photos, and scan codes."
---

`@ink/camera` provides camera permission, photo capture, and code scanning. The preview uses Ink's standard `Screen` header and fills the remaining content area.

## Permission

`cameraPermission()` returns `"granted"`, `"denied"`, `"blocked"` or `"unknown"` when ready.

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

Creating the resource does not open a prompt. On a Light Phone III, Ink uses the LightOS permission screen. On an ordinary Android development device, it uses the Android permission prompt.

## Take a photo

Create a session with `photoCapture()` and pass it to `CameraPreview`. The preview must be the only child of its `Screen`.

```tsx
import { CameraPreview, photoCapture } from "@ink/camera";
import { Screen } from "ink";

export default function Photo() {
  const capture = photoCapture();

  return (
    <Screen title="Photo">
      <CameraPreview session={capture} />
    </Screen>
  );
}
```

Opening the screen starts the preview. Tap anywhere on the preview to capture. The preview then shows **Retake** and **Use photo**. The session becomes `ready` only after the photo is accepted.

The ready value contains:

- `source`, an opaque image accepted by `Image.src`;
- pixel `width` and `height`;
- `mimeType` as `"image/jpeg"`;
- `capturedAtMs` as Unix time in milliseconds.

The source cannot be read as a filesystem path or persisted in Ink state. One accepted photo is kept for the active session; accepting another replaces it.

```tsx
import { Image } from "ink";

{capture.status === "ready" ? (
  <Image
    src={capture.value.source}
    width={349}
    height={349}
    fit="contain"
  />
) : null}
```

## Scan a code

`codeScanner()` scans QR codes by default. Pass a literal format list to accept other code types.

```tsx
import { CameraPreview, codeScanner } from "@ink/camera";
import { Screen } from "ink";

export default function Scan() {
  const scanner = codeScanner({ formats: ["qr", "ean-13", "code-128"] });

  return (
    <Screen title="Scan code">
      <CameraPreview session={scanner} />
    </Screen>
  );
}
```

Supported formats are QR, Aztec, Data Matrix, PDF417, Codabar, Code 39, Code 93, Code 128, EAN-8, EAN-13, ITF, UPC-A, and UPC-E.

The first recognised code stops scanning. The ready value contains `text` and `format`. A scan that remains active for 60 seconds returns a timeout error.

## Session states

A photo or scanner session is:

| Status | Meaning |
| --- | --- |
| `idle` | The preview is not active. |
| `opening` | The camera is starting. |
| `active` | The preview is ready for capture or scanning. |
| `ready` | A photo was accepted or a code was found. |
| `error` | The session failed. |

`open()` retries or reopens a session. It never requests permission.

Leaving the screen, pressing the standard back button, or moving the app to the background releases the camera. Only one camera session can be open; another active session receives a `busy` error. Late capture and scan results are ignored after a session closes.

Errors distinguish permission, unavailable camera, busy camera, capture, storage, decoding, timeout, protocol, and unexpected failures.

## Packaging

Ink packages camera capabilities independently. Permission handling does not add capture or scanning. Photo capture does not add code decoding. An app that does not import `@ink/camera` carries no camera implementation or manifest entries.
