---
title: "Media"
description: "Pick, inspect, transform, and present photos and video."
---

`@ink/media` picks photos and video into app-readable files, reads their metadata, creates image variants and video thumbnails, and presents them in a native media view.

## Pick media

Create a picker and open it from a user action:

```tsx
import { mediaPicker } from "@ink/media";
import { Button, Text, match } from "ink";

const picker = mediaPicker({
  kinds: ["image", "video"],
  selection: { maximum: 4 },
});

{match(picker, {
  idle: () => <Button onPress={() => picker.pick()}>Choose media</Button>,
  picking: () => <Text>Choosing media</Text>,
  ready: (result) => <Text>{result.items.length} items selected</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

`kinds` accepts `"image"`, `"video"`, or both. `selection` is `"single"` by default. A multiple selection accepts up to 32 items.

A successful pick replaces the previous result. Closing the picker without choosing anything returns it to `idle`. Call `clear()` to release the selected files.

Each selected item contains:

| Field | Value |
| --- | --- |
| `file` | An opaque `FileHandle` accepted by Ink file and media APIs. |
| `kind` | `"image"` or `"video"`. |
| `mimeType` | The validated media type. |
| `name` | A normalised file name. |
| `sizeBytes` | File size in bytes. |
| `width`, `height` | Oriented dimensions in pixels. |
| `durationMs` | Video duration, or `null` for an image. |
| `capturedAtMs` | Capture time when available. |
| `orientation` | Source rotation as `0`, `90`, `180`, or `270`. |

The file handle has no filesystem path or Android URI. Use `managedFile().replace()` from `@ink/files` when a picked file must survive after the picker is cleared or its screen leaves.

## Inspect a file

Use `mediaInfo()` to validate an existing file and read the same metadata:

```tsx
import { mediaInfo } from "@ink/media";
import { Text, match } from "ink";

const info = mediaInfo(file);

{match(info, {
  loading: () => <Text>Reading media</Text>,
  ready: (result) => (
    <Text>{result.value.width} × {result.value.height}</Text>
  ),
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

The resource reloads when you call `reload()`. It stops reading when its screen leaves and ignores late results from a replaced request.

## Transform an image

`mediaTransform()` creates a new file and leaves the source unchanged:

```tsx
import { mediaTransform } from "@ink/media";
import { Button, Text, match } from "ink";

const transform = mediaTransform();

<Button onPress={() => transform.run(photo.file, {
  kind: "image",
  maxWidth: 1200,
  maxHeight: 1200,
  fit: "contain",
  format: "jpeg",
  quality: 85,
})}>
  Prepare photo
</Button>

{match(transform, {
  idle: () => null,
  running: () => <Text>Preparing photo</Text>,
  ready: (result) => <Text>{result.value.sizeBytes} bytes</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

Image transforms support `jpeg`, `png`, and `webp`. Width and height accept 1 to 8,192 pixels. `quality` accepts 1 to 100 for lossy formats.

`fit: "contain"` keeps the complete image and never enlarges it. `fit: "cover"` fills the requested dimensions and crops centrally. Both apply source orientation before resizing.

Create a video thumbnail with this recipe:

```tsx
transform.run(video.file, {
  kind: "video-thumbnail",
  positionMs: 10_000,
  maxWidth: 640,
  maxHeight: 360,
  format: "jpeg",
  quality: 80,
});
```

The position is clamped to the video duration. Call `cancel()` to stop the current transform or `clear()` to release its result.

## Present media

Create a presentation controller and pass it to `MediaView`:

```tsx
import { MediaView, mediaPresentation } from "@ink/media";

const presentation = mediaPresentation(video.file);

<MediaView
  presentation={presentation}
  width={349}
  height={240}
  fit="contain"
  controls="video"
  accessibilityLabel="Interview with Sam"
/>
```

`fit` is `"contain"` by default or `"cover"` to fill and crop. `controls="video"` adds play, pause, seek, elapsed-time and mute controls. It has no effect on images.

The controller exposes `play()`, `pause()`, `seekTo(positionMs)`, `setMuted(muted)` and `retry()`. Its status is `opening`, `ready`, `playing`, `paused`, `ended`, or `error`.

Pass `autoplay`, `loop`, or `muted` to `mediaPresentation()` when needed. Autoplay never begins with sound. Video pauses when its screen leaves or the app enters the background. Removing the view stops its presentation.

## Permissions and errors

Picking uses a system-owned selection screen and does not request broad photo-library or storage permission. The package does not request camera or microphone permission.

Errors distinguish cancellation after transfer starts, denied access, missing files, unsupported or invalid media, size limits, storage, decoding, playback, and unexpected failures. Every error has `kind`, `message`, `retryable`, and an `operation` such as `"pick"`, `"inspect"`, `"transform"`, `"open"`, or `"play"`.

## Accessibility

Give every `MediaView` an `accessibilityLabel` that describes its content. File names are not used as descriptions.

Video controls expose their roles, current time, duration, and seek progress to assistive technology. Provide adjacent text for information conveyed only by an image or video. Avoid autoplay for content that could distract from the current task.
