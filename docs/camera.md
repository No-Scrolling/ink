---
title: "Camera"
description: "Native preview and capture owned by the visible screen."
tag: "Design specification"
---

`@ink/camera` provides a native preview and still capture. Use it for a chat photo, a document image or an app-specific capture flow.

```tsx
import { Button, Screen, Text, useAction } from "ink";
import { CameraPreview, useCamera } from "@ink/camera";

export function Capture() {
  const camera = useCamera({ facing: "back" });
  const capture = useAction(() => camera.capture());
  return (
    <Screen title="Photo">
      <CameraPreview controller={camera} />
      <Button onPress={() => capture.run()} disabled={!camera.state.ready}>Take photo</Button>
      {capture.status === "error" && <Text>{capture.error.message}</Text>}
    </Screen>
  );
}
```

The hook activates after commit when the screen is visible. Permission states are exposed separately from readiness; render a permission explanation and explicit `camera.requestPermission()` action when needed. Merely constructing the hook does not show a permission prompt.

Preview, focus and image processing remain native. `capture()` returns a temporary managed file with dimensions and orientation metadata. Promote it with [Files](files.md) before saving a durable record or leaving it in an outbox.

The camera releases when its screen is covered or the app backgrounds, and reacquires on return. Interrupted capture rejects or completes with a result tied to the original request; it cannot update a replacement screen. Handle denial, camera-in-use and unavailable hardware distinctly.

Use [Barcode](barcode.md) for code scanning and [Media](media.md) for choosing an existing photo. Apps should not build a JavaScript frame-processing loop merely to decode a code.
