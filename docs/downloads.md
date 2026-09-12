---
title: "Downloads"
description: "Download files and track progress after a screen closes."
---

Use `@ink/network/downloads` to download files after a screen closes. Ink tracks progress, manages partial files and recovers interrupted transfers.

## Start a download

```ts
import { downloads } from "@ink/network/downloads";

const download = await downloads.enqueue({
  key: `episode:${episode.id}`,
  url: episode.audioUrl,
  name: episode.title,
  network: "unmetered",
});
await saveEpisodeDownload(episode.id, download.id);
```

`episode` and `saveEpisodeDownload` come from your app.

### Avoid duplicates

Reusing a `key` avoids duplicate downloads. The same key with different source details rejects; use a new key when content changes.

## Observe progress

Inside your component, observe a saved `downloadId`:

```ts
import { useMemo } from "react";
import { useSnapshot } from "ink";
import { downloads } from "@ink/network/downloads";

const source = useMemo(() => downloads.observe(downloadId), [downloadId]);
const download = useSnapshot(source);
```

Keep the source stable while the ID is unchanged.

### Read transfer state

The snapshot is `loading`, `ready` or `error`. Ready data includes:

- Transfer state: `queued`, `running`, `paused`, `completed`, `failed` or `cancelled`.
- Bytes received and the total size, when known.
- A [FileRef](/files#file-metadata) after completion, or an error if the transfer failed.

A snapshot error means Ink could not read the job. A failed transfer appears within ready job data.

To read a saved job once:

```ts
const download = await downloads.get(downloadId);
```

## Pause and resume

Use the saved `downloadId`:

```ts
await downloads.pause(downloadId);
```

To resume:

```ts
await downloads.resume(downloadId);
```

## Cancel a download

Cancel unfinished work and delete its partial files:

```ts
await downloads.cancel(downloadId);
```

## Remove a download

Delete the job and all its files, including a completed download:

```ts
await downloads.remove(downloadId);
```

Discard the ID after removal. Closing a screen or stopping observation does not cancel or remove a download.

## Interrupted downloads

Ink saves transfer state across app restarts. It resumes partial files when the server supports it and the content still matches. Otherwise, it restarts the transfer.

Network constraints and storage failures appear in state. Handle missing files when reopening downloads.

Android schedules transfers and restores pending work after reboot. Force-stopping the app prevents background work until it opens again.

## Download URLs

Release builds require HTTPS. Debug builds also accept loopback HTTP.

### Authentication and expiry

Downloads do not add authentication headers. Use an authorised URL that remains valid for retries.

If a signed URL expires, obtain a replacement and enqueue a new request. Do not put permanent bearer tokens in saved URLs.

Protected offline media may require a provider’s own download and playback integration.

Use `fetch` for short foreground requests and [Files](/files) for uploads. Background jobs can check download state; they should not run a continuous download loop.
