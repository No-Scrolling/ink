# Camera

Import camera capabilities from `@ink/camera`. The module keeps permission handling, CameraX lifecycle, photo storage and code decoding behind three compile-time constructors; applications do not receive an Android camera object or run JavaScript callbacks.

## Permission

`cameraPermission()` is a screen-scoped resource with `loading`, `ready` and `error` states. Its ready value is `"granted"`, `"denied"`, `"blocked"` or `"unknown"`.

```tsx
const permission = cameraPermission();

<Button onPress={() => permission.request()}>Request Camera</Button>
```

Construction never opens a prompt. `request()` must be called by a user action. On LightOS, Ink uses the Light SDK permission activity and honours a server block without falling through to an Android prompt. On an ordinary Android development device, Ink uses the platform permission request.

## Photos

`photoCapture()` creates a screen-scoped session. Mount `CameraPreview` as the only child of a `Screen`; entering that screen activates the session and fills the complete content area beneath Ink's standard header. Tap anywhere on the live preview to capture. Ink then renders **Retake** and **Use photo** inside the same layout node; only accepting the review produces a ready value.

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

`CapturedPhoto` contains an opaque `source`, pixel width and height, the fixed MIME type `image/jpeg`, and `capturedAtMs`. The source is accepted only by `Image.src`; it cannot be inspected, serialised, persisted or turned into a filesystem path. Ink keeps one accepted app-private photo for each live session. Accepting another replaces it.

`CameraPreview` is a genuine Ink layout node, not a second screen or an Android overlay with a copied header. Ink measures the node and reports its final physical-pixel rectangle and session identity to the Android adapter. Android places only CameraX's hardware-backed `PreviewView` over that rectangle. Camera frames stay on CameraX's surface path rather than being copied through JNI and uploaded to WGPU; this avoids continuous CPU copies, allocations and GPU texture uploads on constrained hardware. Ink continues to render and own the header, navigation, review and accepted-photo states.

## Code scanning

`codeScanner()` defaults to QR codes. An optional literal format list can enable Aztec, Data Matrix, PDF417, Codabar, Code 39/93/128, EAN-8/13, ITF and UPC-A/E.

```tsx
import { CameraPreview, codeScanner } from "@ink/camera";
import { Screen } from "ink";

export default function Scan() {
  const scanner = codeScanner({ formats: ["qr", "ean-13"] });

  return (
    <Screen title="Scan Code">
      <CameraPreview session={scanner} />
    </Screen>
  );
}
```

The scanner uses the same preview seam. The first recognised result releases the camera and the node shows the decoded value. A scan that remains active for 60 seconds fails with `timeout`.

## Session semantics

A session is `idle`, `opening`, `active`, `ready` or `error`. Errors expose a stable kind, message and retryable flag. Kinds are `permission-denied`, `permission-blocked`, `unavailable`, `busy`, `capture`, `storage`, `decoder`, `timeout`, `protocol` and `unexpected`.

- Mounting `CameraPreview` on the active screen activates its session automatically. Navigation into that screen remains the explicit author-controlled user action.
- `open()` remains available for an Ink control that deliberately retries or reopens a session; it never requests permission automatically.
- Ink's standard back button, application pause and leaving the owning screen release CameraX.
- Leaving before a result restores the previous ready value, or `idle` when none exists.
- Photo review releases the camera; **Retake** creates a fresh preview and CameraX binding for the same Ink session.
- Only one camera session can be open. A competing session reports `busy`.
- Late provider, capture and decode results are ignored after close.

## Packaging

Ink's generated Android feature flags are independent:

- Permission only adds the camera manifest permission, optional hardware declaration and the small permission adapter.
- Photo capture adds CameraX and the photo implementation, but not ZXing.
- Code scanning adds CameraX, ZXing Core and the scanner implementation, but not photo capture code.
- An application that does not import `@ink/camera` carries none of these sources, dependencies or manifest entries.

CameraX is pinned to `1.5.0` and ZXing Core to `3.5.4`, matching the reviewed Light SDK dependency versions.
