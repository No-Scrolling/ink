---
title: "Files and media"
description: "Choose attachments and use the same managed file throughout an app."
---

Use managed files to preview, store and upload attachments. Choose the import for the operation you need:

| Import | Provides |
| --- | --- |
| `@ink/files` | Pick documents, open, remove, save and share files. |
| `@ink/files/images` | Prepare images for display or upload. |
| `@ink/files/media` | An Ink photo and video gallery with thumbnail display. |

Basic file operations do not include image preparation or Ink’s image renderer. Import `Image` from `ink` to display images. The gallery includes thumbnail rendering without image preparation.

## Pick an attachment

`MediaPicker` is a complete gallery screen with a three-column grid, automatic pagination and multi-selection. Put it on its own route:

```tsx
import { MediaPicker } from "@ink/files/media";
import type { FileRef } from "@ink/files";

export function Attachments({ onSelect }: { onSelect: (files: FileRef[]) => void }) {
  return <MediaPicker onSelect={onSelect} />;
}
```

The gallery requests full library access on entry. Android owns the permission prompt; Ink owns the gallery, square thumbnails, scrolling and selection checks. It displays photos and videos by default; `kind="image"` or `kind="video"` narrows the library. `title` optionally changes the header. Denied access offers a retry; blocked access offers settings and a fresh permission check. Partial access to selected photos does not satisfy the full-library permission required by this component.

Tap thumbnails to select or deselect them, then press the header check. Selected thumbnails dim beneath a larger white check in the centre. The component creates durable app-owned copies of every selected item before calling `onSelect(FileRef[])`. The callback may return a promise. It receives files in gallery order and owns them from that point, including retention if its own processing fails. The app decides when to navigate back, preview or upload them. Back cancels selection; disposal cancels unfinished imports and removes copies that have not been handed to the app.

Rows are virtualised through Ink's List. Ink prepares the next page of metadata and loads thumbnails up to one viewport ahead and behind. Page queries run separately from thumbnail decoding. This reduces gaps during scrolling without loading the whole library into memory. Importing keeps the original media.

Selected items use a filled Check Circle; videos have a filled Video File icon. Importing `@ink/files/media` adds the media-library permission capability; document picking and other `@ink/files` commands do not require broad photo access.

For documents, `await files.pick({ types: ["application/pdf"] })` opens the platform picker and returns one durable `FileRef`, or `null` on cancellation. It owns the external activity round trip.

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

Accepted camera photos (`photo.file`), completed recordings and downloads use this representation. Camera results retain their earlier `uri` and `source` aliases for compatibility.

Persist the file ID alongside app data. `files.open(id)` returns current metadata or `null` if removed; storage failures reject. Accepted files survive screen disposal and app restarts until explicitly removed or app data is cleared. No public temporary-file promotion step is required.

`files.remove(id)` deletes an app-owned file. The app owns retention: deleting one message should not remove a file referenced elsewhere. Failed operations clean up their partial files.

A successful native import makes the file durable. Cancellation before that success cleans up the copy; later cancellation does not revoke ownership. As with document picking, process death between native success and the app recording the returned ID can leave an app-owned file without an app record. Import and app-data persistence are not one transaction.

## Preview, prepare and upload

`Image` and audio playback accept appropriate file `src` values. Media bytes stay native during display. Picking a video does not provide video playback.

`prepareImage` returns a new managed image with orientation applied and dimensions bounded while retaining aspect ratio. The original remains intact. Prepared output omits location metadata.

```ts
import { prepareImage } from "@ink/files/images";

const smaller = await prepareImage(file, { maxWidth: 1024, maxHeight: 1024 });
```

Import `"@ink/network"` before using `fetch`, `Blob` or `FormData`. `fetch(file.src)` reads the local file as a native-backed response. Its `blob()` can be used as a request body or appended to `FormData` without copying the complete file into the JavaScript heap. Native upload preparation copies attachment ranges into an upload spool, preserving multipart boundaries and replay after redirects. Explicit `text()` and `arrayBuffer()` calls materialise bytes; use `body` for incremental reading. Apps supply endpoints, authentication and upload state.

`files.save(file)` lets the user save an external copy. `files.share(file)` opens an external share destination with temporary Android access grants. Cancellation leaves the original intact.

The template's Files and media screen keeps attachment IDs in a store, reopens them after a restart, previews images and prepares a smaller copy. Its upload action uses the [local account and transfer fixture](https://github.com/vandamd/ink/blob/main/examples/light-template/scripts/auth-server.md): run `node examples/light-template/scripts/auth-server.mjs`, then use a debug Android emulator build. The example uploads multipart data to `http://10.0.2.2:8788/upload`. Save and share open Android interfaces from explicit button actions.

## Scope

Arbitrary filesystem paths, document reading, general image editing and video playback are not supported. Use [Downloads](/downloads) for files that should keep downloading after a screen closes.
