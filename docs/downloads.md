---
title: "Downloads"
description: "Durable native transfers for offline content."
tag: "Design specification"
---

`@ink/downloads` owns transfer scheduling, progress, temporary files and completion. Use it for podcast episodes, tickets, reader documents and supported media-provider content.

```ts
import { downloads } from "@ink/downloads";

const download = await downloads.enqueue({
  key: `episode:${episode.id}`,
  url: episode.audioUrl,
  name: episode.title,
  network: "unmetered",
});
await saveEpisodeDownload(episode.id, download.id);
```

This fragment assumes the app's episode model and record operation. A stable key deduplicates the same requested asset; it must include a revision when content at an ID changes. Persist the download ID before relying on its presence in the library.

## Observe and control

`downloads.observe(id)` is a readable source for `useSnapshot`. States are queued, running, paused, completed, failed or cancelled. Progress includes received bytes and an optional total; unknown content length must not display a fabricated percentage.

`pause(id)`, `resume(id)`, `cancel(id)` and `remove(id)` return promises. Cancellation stops pending transfer work and removes partial content. Removal also deletes completed managed content. Releasing an observer does neither.

A completed snapshot contains a managed file ID. The library remains responsible for metadata, playback eligibility and storage policy. Missing or removed files require reconciliation on opening the app.

## Recovery and credentials

Native persisted transfer state supports recovery after process restart. Resuming a partial transfer depends on server range support and validators; changed content restarts safely. Android constraints and storage pressure can delay or fail a transfer.

Do not persist an expiring bearer token as a permanent URL/header recipe. Provider modules use a registered headless resolver that obtains fresh credentials by account ID when required. If renewal needs user input, expose an account-required state and resume after sign-in.

Secrets remain in secure storage; download metadata and diagnostic URLs are redacted where the package knows they contain credentials. Apps must still avoid putting secrets into names and arbitrary metadata.

A normal fetch suits short foreground reads. Use downloads when transfer ownership must survive leaving the screen, and [Files](files.md) for completed content.
