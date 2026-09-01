---
title: "Camera"
description: "Capture photos or video through a native camera session."
tag: "Partial"
---

`@ink/camera` captures photos and video through one native camera session. `CameraView` owns the standard capture, review, retake, and acceptance interface; the session exposes commands for custom layouts.

## Capture a photo

Create a session and pass it to `CameraView`:

```tsx
import { CameraView, cameraCapture } from "@ink/camera";
import { Screen } from "ink";

export default function Photo() {
  const camera = cameraCapture({
    kind: "photo",
    facing: "back",
    permissionPrompt: "Allow camera to take a photo",
  });

  return (
    <Screen title="Photo">
      <CameraView session={camera} />
    </Screen>
  );
}
```

The standard view shows an explicit permission action when needed. Creating the session never opens a prompt. On Light Phone III, Ink uses the LightOS permission screen; ordinary Android development devices use the Android prompt.

After capture, the view shows **Retake** and **Use photo**. Accepting produces:

- `source`, an opaque image accepted by `Image.src`;
- `file`, a temporary `FileHandle` accepted by Files and Media;
- pixel `width` and `height`;
- `mimeType` and `capturedAtMs`.

Call `file("photos/profile.jpg").replace(result.file)` when the capture must become durable.

## Record video

Use the same interface with `kind: "video"`:

```tsx
const camera = cameraCapture({
  kind: "video",
  facing: "back",
  audio: true,
  maximumDurationMs: 60_000,
  permissionPrompt: "Allow camera and microphone to record video",
});

<CameraView session={camera} />
```

Camera owns the composed camera and microphone permission flow for video. Callers do not need to coordinate Audio permission separately.

The accepted value adds `durationMs` and `sizeBytes`. `maximumDurationMs` accepts 1 second to 30 minutes and defaults to 5 minutes. Video quality is `"compact"`, `"balanced"`, or `"maximum"`.

## Build a custom camera interface

Use `camera.permission` and its `request()` command when the standard view is not appropriate. The session also provides `capture()`, `startRecording()`, `stopRecording()`, `retake()`, `accept()`, `focus()`, `setZoom()`, `setTorch()`, and `switchFacing()` where supported.

The session publishes one complete snapshot with a domain `phase`: `idle`, `opening`, `active`, `capturing`, `recording`, `reviewing`, `ready`, or `error`. Commands are serialised. Unsupported commands return an action error without closing the session.

## Lifecycle and errors

Leaving the screen or moving the app to the background releases the camera and discards an unaccepted capture. An accepted temporary handle remains valid for the session lifetime. Consumers acquire a lease before the session releases it.

Only one camera-backed session can be active, including Barcode scanners. A second session receives `busy`.

Errors distinguish denied or blocked permission, unavailable or busy hardware, unsupported controls, capture and recording failures, storage and size limits, interruption, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.

Use [Barcode](barcode.md) to scan codes. Barcode generation does not link camera capability.
