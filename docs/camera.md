---
title: "Camera"
description: "Request camera access and capture photos or video."
---

`@ink/camera` captures photos and video through a native camera view. The preview uses Ink's standard `Screen` header and fills the remaining content area.

## Request permission

`cameraPermission()` returns `"granted"`, `"denied"`, `"blocked"`, or `"unknown"` when ready.

```tsx
import { cameraPermission } from "@ink/camera";
import { Button, Text, match } from "ink";

const permission = cameraPermission();

{match(permission, {
  loading: () => <Text>Checking camera permission</Text>,
  ready: ({ value }) => <Text>{value}</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
<Button onPress={() => permission.request()}>Allow camera</Button>
```

Creating the resource does not open a prompt. On a Light Phone III, Ink uses the LightOS permission screen. On an ordinary Android development device, it uses the Android permission prompt.

Video with sound also needs microphone permission from `@ink/audio`. Request it before opening a video session with `audio: true`.

## Take a photo

Create a controller with `photoCapture()` and pass it to `CameraView`:

```tsx
import { CameraView, photoCapture } from "@ink/camera";
import { Screen } from "ink";

export default function Photo() {
  const camera = photoCapture({ facing: "back" });

  return (
    <Screen title="Photo">
      <CameraView controller={camera} />
    </Screen>
  );
}
```

Tap the capture control to take a photo. The view then shows **Retake** and **Use photo**. The controller becomes `ready` after the user accepts it.

The ready value contains:

- `source`, an opaque image accepted by `Image.src`;
- `file`, a temporary `FileHandle` accepted by `@ink/files` and `@ink/media`;
- pixel `width` and `height`;
- `mimeType` as `"image/jpeg"`;
- `capturedAtMs` as Unix time in milliseconds.

Use `managedFile().replace(result.file)` from `@ink/files` when the photo must survive after the camera screen leaves.

Photo options are:

| Option | Values | Default |
| --- | --- | --- |
| `facing` | `"back"`, `"front"` | `"back"` |
| `flash` | `"off"`, `"auto"`, `"on"` | `"off"` |
| `quality` | `"balanced"`, `"maximum"` | `"balanced"` |

## Record video

Create a video controller and pass it to the same view:

```tsx
import { CameraView, videoCapture } from "@ink/camera";

const camera = videoCapture({
  facing: "back",
  audio: false,
  maximumDurationMs: 60_000,
});

<CameraView controller={camera} />
```

Call `start()` and `stop()` from buttons or use the view's standard record control. The controller stops automatically at `maximumDurationMs` or its configured size limit.

The ready value contains a temporary `FileHandle`, `mimeType`, pixel dimensions, `durationMs`, `sizeBytes`, and `capturedAtMs`. Pass the file to `@ink/media` for playback, metadata, or a thumbnail.

`maximumDurationMs` accepts 1,000 milliseconds to 30 minutes and defaults to 5 minutes. `quality` is `"compact"`, `"balanced"`, or `"maximum"`.

## Control focus, zoom, and torch

`CameraView` supports tap-to-focus and pinch-to-zoom. The controller also exposes:

- `focus({ x, y })` with logical coordinates inside the view;
- `setZoom(value)` from `1` to the reported `maximumZoom`;
- `setTorch("off" | "on")` while the back camera is active;
- `switchFacing()` when both cameras are available.

Unsupported controls return an `unsupported-control` error without closing the session.

## Session states

A photo or video controller is:

| Status | Meaning |
| --- | --- |
| `idle` | The camera view is not active. |
| `opening` | The camera is starting. |
| `active` | The preview is ready. |
| `capturing` | A photo is being processed. |
| `recording` | Video recording is active. |
| `reviewing` | The view is presenting captured media for approval. |
| `ready` | The user accepted the captured media. |
| `error` | The session failed. |

`open()` retries a failed session. `retake()` removes the temporary capture and returns to `active`. `accept()` publishes the ready value.

## Lifecycle and errors

Leaving the screen, pressing the standard back button, or moving the app to the background releases the camera and discards an unaccepted capture. An accepted temporary file remains valid for its controller lifetime.

Only one camera-backed controller can be active. This includes barcode scanners from `@ink/barcode`. A second controller receives a `busy` error.

Errors distinguish denied or blocked permission, unavailable or busy camera, unsupported controls, capture, recording, microphone, storage, size limits, interrupted sessions, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.

Use [Barcode](barcode.md) to scan codes. Barcode generation does not include camera capability.
