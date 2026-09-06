---
title: "Camera"
description: "Native preview and capture owned by the visible screen."
---

Use `@ink/camera` to preview, capture and review photos. It supports front and rear cameras when the device provides them.

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

Tap the preview to capture a photo; the review offers Retake and Use photo actions. On acceptance, `camera.state.value` contains `source`, `width`, `height`, `mimeType` and `capturedAt`. Its `file` is a shared `FileRef` with `id`, `src`, `name`, `size`, `mimeType` and dimensions. Accepted files live in private app storage and survive screen disposal and app restarts. Store the ID alongside your record and remove the file explicitly with `files.remove(photo.file.id)` from `@ink/files`. The earlier `uri`, `source` and `camera.removePhoto` remain available for compatibility. Unaccepted review files are temporary and are removed when the session closes.

Call `await controller.capture()` to trigger the same capture-and-review flow as tapping the preview. This promise acknowledges the command; it does not imply that the user accepted a photo. Read `controller.state.reviewSource` for review readiness and `controller.state.value` after acceptance. `controller.accept()` and `controller.retake()` also control that flow. A preview must be mounted and the controller ready before capture.

Use `useCamera({ facing: "front" })` to request the front camera. Missing hardware reports an unavailable error rather than silently choosing a different camera.

The camera releases when its screen is covered or the app backgrounds, and reacquires on return. Interrupted capture rejects or completes with a result tied to the original request; it cannot update a replacement screen. Handle denial, camera-in-use and unavailable hardware distinctly.

Use `@ink/camera/scan` for [code scanning](barcode.md); still-capture imports do not include the scanner. Choose an existing photo with [Files and media](files.md). Apps should not build a JavaScript frame-processing loop merely to decode a code.
