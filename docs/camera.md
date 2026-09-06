---
title: "Camera"
description: "Native preview and capture owned by the visible screen."
---

Front/rear preview, capture, review and durable photo files are supported. Camera availability and quality depend on the device.

`@ink/camera` provides a native preview and still capture. Use it for a chat photo, a document image or an app-specific capture flow.

```tsx
import { Screen } from "ink";
import { CameraPreview, useCamera } from "@ink/camera";

export function Capture() {
  const camera = useCamera({ facing: "back" });
  return (
    <Screen title="Photo">
      <CameraPreview controller={camera} />
    </Screen>
  );
}
```

The hook activates after commit when the screen is visible. Permission states are exposed separately from readiness; render a permission explanation and explicit `camera.requestPermission()` action when needed. Merely constructing the hook does not show a permission prompt.

Tap the preview to capture a photo; the review offers Retake and Use photo actions. On acceptance, `camera.state.value` contains `source`, `width`, `height`, `mimeType` and `capturedAt`. The result also contains `file` with `uri`, `source`, `name`, `size` and `mimeType`. Accepted files live in private app storage and survive screen disposal and app restarts. Store this metadata alongside your record; remove a file explicitly with `await camera.removePhoto(photo.file)` from the exported `camera` object. Unaccepted review files are temporary and are removed when the session closes.

Call `await controller.capture()` to trigger the same capture-and-review flow as tapping the preview. This promise acknowledges the command; it does not imply that the user accepted a photo. Read `controller.state.reviewSource` for review readiness and `controller.state.value` after acceptance. `controller.accept()` and `controller.retake()` also control that flow. A preview must be mounted and the controller ready before capture.

Use `useCamera({ facing: "front" })` to request the front camera. Missing hardware reports an unavailable error rather than silently choosing a different camera.

The camera releases when its screen is covered or the app backgrounds, and reacquires on return. Interrupted capture rejects or completes with a result tied to the original request; it cannot update a replacement screen. Handle denial, camera-in-use and unavailable hardware distinctly.

Use [Barcode](barcode.md) for code scanning. Choosing an existing photo is covered by the planned [Files and media](files.md) capability. Apps should not build a JavaScript frame-processing loop merely to decode a code.
