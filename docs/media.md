---
title: "Media"
description: "Pick, inspect, transform, and present photos and video."
tag: "Planned"
---

`@ink/media` owns a media file's validated metadata, transformation actions, and native presentation. It accepts temporary Files or Camera handles and durable Files or Downloads references.

## Pick media

Use `mediaPicker()` from a user action:

```tsx
const picker = mediaPicker({
  kinds: ["image", "video"],
  maximumItems: 4,
  maximumItemBytes: 20_000_000,
  maximumTotalBytes: 50_000_000,
});

<Button onPress={() => picker.run()}>Choose media</Button>
```

The action uses `idle`, `running`, `success`, and `error`. A successful value contains validated media items with a temporary `FileHandle`, kind, MIME type, normalised name, dimensions, size, optional duration, orientation, and capture time.

Closing the system picker returns to `idle`. `clear()` releases every temporary item after active consumers finish their leases. Use `file(...).replace(item.file)` when an item must become durable.

## Open media

Create one media session from a handle or durable reference:

```tsx
const clip = media(videoFile, {
  autoplay: false,
  loop: false,
  muted: false,
});

{clip.phase === "ready" ? (
  <Text>{clip.info.width} × {clip.info.height}</Text>
) : null}
```

Opening validates the source and publishes complete metadata once. The session acquires a lease for its lifetime, so a temporary owner can be disposed without interrupting an active consumer.

## Transform media

Run a transformation through the media session:

```tsx
<Button onPress={() => clip.transform({
  kind: "image",
  maximumWidth: 1200,
  maximumHeight: 1200,
  fit: "contain",
  format: "jpeg",
  quality: 85,
})}>
  Prepare image
</Button>
```

The transformation action reports progress when the codec provides it. Its success contains a temporary media handle. Pass `destination: managed.reference` to commit output atomically to an existing managed file.

Image transformations support JPEG, PNG, and WebP. Video thumbnail transformations accept a position and image output options. Ink stops observing immediately after `cancel()`; terminating codec work is best effort and late output is discarded.

## Present media

Attach the same session to `MediaView`:

```tsx
<MediaView
  session={clip}
  height={240}
  fit="contain"
  controls="video"
  accessibilityLabel="Interview with Sam"
/>
```

The view fills available width and uses explicit height. Video controls provide play, pause, seek, elapsed time, and mute. The session also exposes ordered `play()`, `pause()`, `seekTo()`, and `setMuted()` commands for custom controls.

Video pauses when hidden or backgrounded. Autoplay never begins with sound.

## Permissions, accessibility, and errors

Picking uses a system-owned surface and requests no broad photo or storage permission. Media does not request Camera or microphone permission.

Errors distinguish denied access, missing or expired files, unsupported or invalid media, per-item and aggregate size limits, storage, decoding, transformation, playback, and unexpected failures. Every error has `kind`, `message`, `retryable`, and `operation`.

Give every `MediaView` an accessibility label that describes its content. Video controls expose their role, duration, current time, and seek progress. Provide adjacent text for meaning conveyed only by visual media.
