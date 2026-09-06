---
title: "Downloads"
description: "Durable HTTP downloads with native progress and recovery."
---

`@ink/downloads` handles ordinary HTTP files that should keep downloading after a screen closes. Ink owns scheduling, progress, partial files and recovery. Apps own the downloaded library and retention policy.

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

The example assumes app-owned episode data and persistence. A stable key identifies one asset revision and deduplicates requests. The same key with conflicting source details rejects; use a new key for replacement content.

## Observe and control

`downloads.observe(id)` supplies Ink's standard `loading`, `ready` or `error` snapshot for `useSnapshot`. Ready data contains the transfer state: `queued`, `running`, `paused`, `completed`, `failed` or `cancelled`. Progress contains received bytes and an optional total. Completion contains a [FileRef](files.md#one-file-representation). Failure to read the job is a snapshot error; a failed transfer is job data with an error explaining the failure.

`pause(id)`, `resume(id)`, `cancel(id)` and `remove(id)` return promises. Cancel stops unfinished work and deletes partial content. Remove also deletes the completed file and the saved job; discard its ID afterwards. Closing a screen or releasing an observer does neither.

Native persisted state supports recovery after process restart. Partial resumption depends on server support and content validators; otherwise the transfer restarts safely. Constraints and storage failures appear in state. Apps reconcile missing files when reopening their library.

Android JobScheduler runs transfers independently of the screen and reschedules persisted work after reboot. Android controls when eligible jobs run; force-stopping the app prevents background work until it is opened again. `downloads.get(id)` reads the current state without subscribing. The template saves its last download ID so reopening the example reconnects to that transfer.

Production URLs require HTTPS. Debug builds also accept loopback HTTP for the template's local fixture server. Downloads do not add authentication headers; use an authorised download URL whose lifetime covers recovery.

## Provider integrations

The initial interface handles URLs that remain usable for the transfer and recovery. Expired signed URLs surface a failure; the app obtains a replacement and enqueues a new request. Do not place a permanent bearer token into a persisted URL.

There is no generic headless credential-resolver registry in this scope. Echo TV's Spotify downloads require its existing provider engine, authentication and offline-content handling. That integration remains separate; ordinary HTTP downloads do not replace it.

Use `fetch` for short foreground requests and [Files and media](files.md) for attachment uploads. Background jobs can schedule reconciliation, but do not act as a continuous download loop.
