---
title: "Camera"
description: "Native preview and capture owned by the visible screen."
---

Use `@ink/camera` to preview, capture, and review photos. It supports front and rear cameras when the device provides them.

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

The hook activates after the visible screen commits. Permission and camera readiness are separate states. When permission is needed, explain why and provide an action that calls `camera.requestPermission()`. Calling the hook does not show a permission prompt.

Tap the preview to capture a photo. The review offers **Retake** and **Use photo** actions. On Android, the preview gives a brief solid flash in the current theme colour (black in dark mode, white in light mode), without a fade. This feedback is built into `CameraPreview`.

The Android review displays the full captured frame at screen resolution, with the image and controls appearing together. The camera encodes and saves the full-resolution JPEG in the background. While `camera.state.saving` is `true`, **Use photo** is disabled and its label changes to **Saving photo**. **Retake** remains available. The review image stays the same when saving finishes.

When the user accepts a photo, `camera.state.value` contains `source`, `width`, `height`, `mimeType`, and `capturedAt`. Its `file` is a shared `FileRef` with `id`, `src`, `name`, `size`, `mimeType`, and dimensions. Accepted files live in private app storage and survive screen disposal and app restarts. Store the ID alongside your record and remove the file explicitly with `files.remove(photo.file.id)` from `@ink/files`. The earlier `uri`, `source`, and `camera.removePhoto` remain available for compatibility. Unaccepted review files are temporary and are removed when the session closes.

Call `await controller.capture()` to trigger the same capture-and-review flow as tapping the preview. The promise resolves when the camera acknowledges the command, before the user accepts a photo. Read `controller.state.reviewSource` for the review image and `controller.state.value` after acceptance. `controller.accept()` and `controller.retake()` also control that flow. Custom review controls should wait until `reviewSource` is present and `saving` is `false` before allowing acceptance. Treat `reviewSource` as a temporary display source, not the saved photo. A preview must be mounted and the controller ready before capture.

Use `useCamera({ facing: "front" })` to request the front camera. Missing hardware reports an unavailable error rather than silently choosing a different camera.

The component releases the camera when another screen covers it or the app enters the background. It reopens the camera when the screen becomes visible again. Interrupted capture rejects or completes with a result tied to the original request; it cannot update a replacement screen. Handle permission denial, camera-in-use errors, and unavailable hardware distinctly.

Use `@ink/barcode/scan` for [code scanning](/barcode); still-capture imports do not include the scanner. Choose an existing photo with [Files and media](/files). Apps should not build a JavaScript frame-processing loop merely to decode a code.
