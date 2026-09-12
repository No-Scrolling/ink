---
title: "Files and media"
description: "Choose, save and upload files."
---

Ink copies selected files into private app storage so you can reopen, display or upload them later.

| Import | Provides |
| --- | --- |
| `@ink/files` | Pick documents, open, remove, save and share files. |
| `@ink/files/images` | Prepare images for display or upload. |
| `@ink/files/media` | An Ink photo and video gallery with thumbnail display. |

Import `Image` from `ink` to display images. Basic file operations omit image rendering and preparation; the gallery includes thumbnail rendering only.

## Permissions

| Permission | Required for |
| --- | --- |
| `photos` | `MediaPicker` with `kind="image"`. |
| `videos` | `MediaPicker` with `kind="video"`. |
| `photos-and-videos` | `MediaPicker` with `kind="all"` (the default). |

[MediaPicker](/media-picker) checks access but does not request it:

```ts
import { lightos } from "@ink/lightos";

const permission = await lightos.requestPermission("photos-and-videos");
```

Display the picker when access is `granted`. It requires full access for the selected media kind; access to selected photos alone is not enough. See [Request a permission](/permissions-guide) for the returned statuses.

Document picking with `files.pick()` uses Android’s file picker and does not need full photo-library access. Managed file operations do not request it either.

## Pick a file

### Photos and videos

Use [MediaPicker](/media-picker) to choose photos and videos. It returns files stored by your app.

### Documents

Open Android’s document picker:

```ts
import { files } from "@ink/files";

const file = await files.pick({ types: ["application/pdf"] });
```

The picker returns to your app with a saved `FileRef`, or `null` if cancelled.

## File metadata

Picking, capturing, recording and downloading files returns a `FileRef`:

| Field | Meaning |
| --- | --- |
| `id` | Stable ID used to reopen or remove the file. |
| `src` | Source used to display, play or upload the file. |
| `name` | Display name. |
| `mimeType` | Content type. |
| `size` | Byte length. |
| `width`, `height` | Dimensions when available for images or video. |
| `duration` | Duration in milliseconds when available for audio or video. |

Camera results expose it as `photo.file` and retain `uri` and `source` aliases for compatibility.

## Reopen a file

Save the ID with your app’s data, then use that saved `fileId` to reopen it:

```ts
import { files } from "@ink/files";

const file = await files.open(fileId);
```

The result is current metadata, or `null` if removed. Storage failures reject.

Files remain available until removed or app data is cleared.

## Remove a file

Remove a file by its saved ID:

```ts
await files.remove(fileId);
```

Check that no other records need the file before removing it.

### Failed or cancelled imports

Failed operations and cancelled imports remove unfinished copies. Completed files remain.

Saving a file and storing its ID are separate operations. If the app stops between them, the file can remain without a matching app record.

## Display a file

Pass `src` to `Image` or the audio player. Displaying media does not copy the file into JavaScript memory. Video playback is not supported.

## Resize an image

`prepareImage` creates a resized copy, corrects orientation and removes location metadata. It preserves the aspect ratio and original file.

```ts
import { prepareImage } from "@ink/files/images";

const smaller = await prepareImage(file, { maxWidth: 1024, maxHeight: 1024 });
```

## Upload a file

Upload a selected or reopened file to your endpoint:

```ts
import "@ink/network";
import type { FileRef } from "@ink/files";

export async function uploadFile(file: FileRef, uploadUrl: string) {
  const local = await fetch(file.src);
  const form = new FormData();
  form.append("file", await local.blob(), file.name);

  const response = await fetch(uploadUrl, { method: "POST", body: form });
  if (!response.ok) throw new Error(`Upload failed (${response.status})`);
}
```

Add authentication if your endpoint requires it. Ink prepares the upload natively without copying the whole file into JavaScript memory.

`text()` and `arrayBuffer()` load file contents into JavaScript memory. Use `body` to read in chunks. See [Network](/network) for transfer limits.

## Export a file

### Save a copy

For a selected or reopened `file`, save an external copy:

```ts
await files.save(file);
```

### Share a copy

Open Android’s share interface:

```ts
await files.share(file);
```

Sharing grants temporary access to the file. Cancelling either action leaves the original unchanged.

## Limitations

Arbitrary filesystem paths, document reading, general image editing and video playback are not supported. Use [Downloads](/downloads) for files that should keep downloading after a screen closes.
