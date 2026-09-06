---
title: "Media"
description: "Pick, inspect and prepare photos, audio and video."
tag: "Planned"
---

> **Planned.** This package is not implemented. The APIs below describe the proposed design.

`@ink/media` provides native media selection and preparation. Use it for a chat attachment, avatar, saved cover or imported audio file.

```ts
import { media } from "@ink/media";

const picked = await media.pick({ kind: "image" });
if (picked) {
  const attachment = await media.prepareImage(picked, {
    maxWidth: 1600,
    maxHeight: 1600,
    stripLocation: true,
  });
  await queueAttachment(attachment.file.id);
}
```

`queueAttachment` is an app operation that promotes temporary content into durable storage before returning. Selection cancellation returns `null`. The picker owns its external activity round trip rather than being cancelled when its UI covers the app.

Preparation applies orientation and bounded resizing natively. Results include a managed file reference, dimensions, MIME type and size. JPEG quality and format conversion are explicit options; do not repeatedly re-encode already suitable content.

## Ownership and rendering

Selections may refer to externally owned content. Import or prepare a managed copy before adding it to a durable outbox. Temporary results require promotion with [Files](files.md) if they must survive cleanup.

Render images with Ink's `Image`. Use `Video` with a supported file or URL for inline video; the native surface owns decoding, seeking and gestures, and pauses when hidden. Full playback controls use the same explicit command/error pattern as [Audio](audio.md).

Metadata reads and thumbnails are native asynchronous operations. Keep large image and video buffers out of React state. An app can display cached thumbnails while the full asset downloads.

For direct capture, use [Camera](camera.md). Provider libraries may impose additional restrictions on formats, upload sizes and media access; validate those before enqueueing an upload.
