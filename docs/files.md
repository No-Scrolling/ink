---
title: "Files and media"
description: "Choose attachments and use the same managed file throughout an app."
tag: "Planned"
---

> **Not implemented yet.** This page defines the intended interface. Current camera and recording results retain their documented shapes until shared attachment support is implemented.

`@ink/files` owns the photo/video picker and managed attachments. Developers can choose a photo, preview it, save its reference and upload it without managing Android content URIs or temporary files.

## Pick an attachment

`MediaPicker` supplies the selection interface inside an Ink screen:

```tsx
import { Screen } from "ink";
import { MediaPicker, type FileRef } from "@ink/files";

export function Attachments({ onSelect }: { onSelect: (file: FileRef) => void }) {
  return (
    <Screen title="Photos and videos">
      <MediaPicker onSelect={onSelect} />
    </Screen>
  );
}
```

The default shows photos and videos; `kind="image"` or `kind="video"` narrows it. Ink owns permission handling, thumbnails, loading, errors and retry. Choosing an item creates a durable app-owned copy before calling `onSelect`. The app decides whether to return to the conversation, preview or upload it. Back cancels selection and cleans up unfinished copies.

For documents, `await files.pick({ types: ["application/pdf"] })` opens the platform picker and returns a durable `FileRef`, or `null` on cancellation. It owns the external activity round trip. Both pickers select one file at a time.

## One file representation

| Field | Meaning |
| --- | --- |
| `id` | Stable app-owned file identity. |
| `src` | Ink-managed source for supported rendering and networking facilities. |
| `name` | Display name. |
| `mimeType` | Content type. |
| `size` | Byte length. |
| `width`, `height` | Dimensions when available for images or video. |
| `duration` | Duration in milliseconds when available for audio or video. |

Accepted camera photos, completed recordings and downloads use this representation in the intended product. Existing capture interfaces need an implementation update to meet it.

Persist the file ID alongside app data. `files.open(id)` returns current metadata or `null` if removed; storage failures reject. Accepted files survive screen disposal and app restarts until explicitly removed or app data is cleared. No public temporary-file promotion step is required.

`files.remove(id)` deletes an app-owned file. The app owns retention: deleting one message should not remove a file referenced elsewhere. Failed operations clean up their partial files.

## Preview, prepare and upload

`Image` and audio playback accept appropriate file `src` values in the intended product. Media bytes stay native during display. Picking a video does not provide video playback.

`files.prepareImage(file, { maxWidth, maxHeight })` returns a new managed image with orientation applied and dimensions bounded while retaining aspect ratio. The original remains intact. Prepared output omits location metadata. This is one preparation operation, not a general image editor.

Managed files must work with standard networking: `fetch(file.src)` reads the local file as a native-backed response. Its `blob()` can be used as a request body or appended to `FormData` without copying the complete file into the JavaScript heap. Implementing native-backed Blob support is part of this work; today's in-memory Blob does not provide this contract. Apps supply endpoints, authentication and upload state. No separate Ink HTTP interface is introduced.

`files.save(file)` lets the user save an external copy. `files.share(file)` opens an external share destination with temporary Android access grants. Cancellation leaves the original intact.

## Scope

This module provides selection, image preparation, shared file references and explicit retention. It does not expose arbitrary filesystem paths, a document reader, a general filesystem toolkit or video playback. [Downloads](downloads.md) owns durable incoming transfers; apps own upload queues and provider rules.
