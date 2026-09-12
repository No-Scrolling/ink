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

[MediaPicker](/media-picker) requests photo and video access when its page opens. There is no separate permission method to call.

It requires full access for the selected media kind. Denied access offers a retry; blocked access offers app settings. Access to selected photos alone is not enough.

Document picking with `files.pick()` uses Android’s file picker and does not need full photo-library access. Managed file operations do not request it either.

## Pick an attachment

Use [MediaPicker](/media-picker) to choose photos and videos. It returns files stored by your app.

Open Android’s document picker:

```ts
import { files } from "@ink/files";

const file = await files.pick({ types: ["application/pdf"] });
```

The picker returns a saved `FileRef`, or `null` if cancelled, then returns to your app.

## One file representation

| Field | Meaning |
| --- | --- |
| `id` | Stable ID used to reopen or remove the file. |
| `src` | Source used to display, play or upload the file. |
| `name` | Display name. |
| `mimeType` | Content type. |
| `size` | Byte length. |
| `width`, `height` | Dimensions when available for images or video. |
| `duration` | Duration in milliseconds when available for audio or video. |

Accepted camera photos (`photo.file`), completed recordings and downloads also return a `FileRef`. Camera results retain `uri` and `source` aliases for compatibility.

## Reopen a file

Save the ID with your app’s data, then use that saved `fileId` to reopen it:

```ts
import { files } from "@ink/files";

const file = await files.open(fileId);
```

The result is the current file metadata, or `null` if removed. Storage failures reject. Files survive screen changes and app restarts until removed or app data is cleared.

## Remove a file

Check that no other records need the file, then remove it by its saved ID:

```ts
await files.remove(fileId);
```

Failed operations remove their partial files.

Cancelling an import removes an unfinished copy, but not a completed file. Saving a file and recording its ID are separate operations: if the app stops between them, the file can remain without a matching app record.

## Preview, prepare and upload

Pass `src` to `Image` or the audio player. Displaying media does not copy the file into JavaScript memory. Video playback is not supported.

`prepareImage` creates a resized copy, corrects its orientation and removes location metadata. The aspect ratio and original file stay unchanged.

```ts
import { prepareImage } from "@ink/files/images";

const smaller = await prepareImage(file, { maxWidth: 1024, maxHeight: 1024 });
```

### Upload files

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

`text()` and `arrayBuffer()` read file contents into JavaScript memory. Use `body` to read in chunks. See [Network](/network) for upload examples and limits. Use your server’s upload endpoint and authentication.

## Save or share a copy

For a selected or reopened `file`, save an external copy:

```ts
await files.save(file);
```

Or open Android’s share interface:

```ts
await files.share(file);
```

Sharing grants temporary access to the file. Cancelling either action leaves the original unchanged.

### Template upload server

The template’s upload example requires a debug emulator build and the [local transfer server](https://github.com/vandamd/ink/blob/main/examples/light-template/scripts/auth-server.md). Start it with `node examples/light-template/scripts/auth-server.mjs`. The example uploads to `http://10.0.2.2:8788/upload`.

## Limitations

Arbitrary filesystem paths, document reading, general image editing and video playback are not supported. Use [Downloads](/downloads) for files that should keep downloading after a screen closes.
