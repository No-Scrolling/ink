---
title: "Downloads"
description: "Download large files reliably into durable app storage."
tag: "Planned"
---

`@ink/downloads` manages resumable, durable file downloads. A download can continue after its screen leaves and can recover after process death without running app JavaScript.

## Create a download

Declare a download with a stable key and call `start()` from a user action:

```tsx
import { download } from "@ink/downloads";
import { Button, Text } from "ink";

const handbook = download({
  key: "handbook.current",
  url: "https://example.com/handbook.epub",
  suggestedName: "handbook.epub",
  constraints: { network: "connected" },
});

<Button onPress={() => handbook.start()}>Download handbook</Button>

{handbook.phase === "downloading" ? (
  <Text>{handbook.progress?.bytesDownloaded ?? 0} bytes downloaded</Text>
) : null}

{handbook.phase === "ready" ? <Text>Available offline</Text> : null}
```

The session phases are `idle`, `queued`, `downloading`, `paused`, `ready`, and `error`. A ready download contains a durable `FileReference` accepted by Files, Media, and Reader. The reference contains no readable path or Android URI.

Declarations with the same key reconnect to the same download and must use compatible options. Package-created keys are automatically namespaced to that package.

## Control a download

Call:

- `start()` to enqueue or resume the transfer;
- `pause()` to stop after the current write settles;
- `cancel()` to cancel work and remove an incomplete file;
- `remove()` to remove the completed file;
- `retry()` after a recoverable error.

`progress` contains `bytesDownloaded`, optional `totalBytes`, and optional `fraction`. Servers do not always provide a reliable content length, so `fraction` can be `null`.

The implementation uses HTTP range requests when the server supports them. Otherwise, it restarts an interrupted transfer. A completed temporary file is moved into durable app storage atomically.

## Authenticate and verify a download

Pass a stable opaque authorisation from Auth or a secret from Secure store. Ink resolves credentials when a request starts and never persists their readable value with download metadata.

```tsx
const archive = download({
  key: "account.archive",
  url: "https://api.example/archive",
  authorization: account.authorization,
  checksum: {
    algorithm: "sha256",
    value: expectedDigest,
  },
});
```

The session waits when its authorisation is unavailable and resumes after Auth restores or refreshes it. A checksum mismatch removes the completed temporary file and returns an `integrity` error.

## Choose constraints

`constraints.network` is `"connected"` or `"unmetered"`. You can also require `charging`, `batteryNotLow`, or `storageNotLow`. Android can delay queued work after every constraint becomes true.

Use Network for small screen-scoped JSON or mutations. Use Downloads when the response is a file, progress matters, or the transfer must survive navigation and process death.

## Lifecycle and errors

Downloads are application-scoped. Android owns transfer execution, Rust owns session state and durable file references, and the app observes the latest snapshot. Cancellation stops observation immediately; terminating the underlying network operation is best effort.

Errors distinguish invalid URLs, offline access, authentication, timeouts, unavailable range support, server failures, insufficient storage, size limits, integrity failures, expired authorisation, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.

The module requests no runtime permission. `ink info` shows each linked download operation and its background and network requirements.
