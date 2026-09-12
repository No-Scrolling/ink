---
title: "Network"
description: "Use fetch with native HTTP transport."
---

Use `@ink/network` for `fetch`, response streams, `Blob`, `File`, `FormData` and `WebSocket`.

Add the package to your dependencies and import it wherever you use these APIs. It includes the native network transport.

```ts
import "@ink/network";

export async function getDepartureBoard(stopId: string, signal?: AbortSignal) {
  const url = new URL("https://api.example.com/departures");
  url.searchParams.set("stop", stopId);
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`Departures unavailable (${response.status})`);
  const body: unknown = await response.json();
  return decodeDepartures(body);
}
```

`decodeDepartures` validates the response for your app. `fetch` rejects network failures and cancellation, but HTTP errors such as 404 still return a response. Check `response.ok` before reading the data.

Use `AbortSignal.timeout()` for timeouts and `AbortSignal.any()` to combine cancellation signals.

## Optional features

The package has separate entry points for [connection state](/connectivity) and [managed downloads](/downloads):

```ts
import { connectivity } from "@ink/network/connectivity";
import { downloads } from "@ink/network/downloads";
```

These imports work independently and do not install `fetch` or other web globals. `@ink/network` alone includes neither feature. Images and maps also load without this import.

## Read a response

`fetch` resolves when headers arrive. Read `response.body` with a reader, async iterator or stream pipeline. Ink reads 32 KiB chunks as needed. Cancel the reader or abort the request when finished.

`json()`, `text()`, `blob()`, `formData()` and `arrayBuffer()` read an entire HTTP response into memory, up to 16 MiB. Stream larger responses instead:

```ts
const response = await fetch(url, { signal });
if (!response.ok || !response.body) throw new Error(`HTTP ${response.status}`);
const reader = response.body.getReader();
try {
  for (;;) {
    const { value, done } = await reader.read();
    if (done) break;
    processChunk(value);
  }
} finally {
  await reader.cancel();
  reader.releaseLock();
}
```

Supply `url`, a cancellation `signal` and a function named `processChunk`. A body can be read once. `clone()` creates a second stream, but unread data can accumulate in memory, so avoid cloning large responses.

Local [managed files](/files) behave differently: `blob()` keeps their data in native storage. `text()` and `arrayBuffer()` still load it into JavaScript memory.

## Upload data

Request bodies accept text, `URLSearchParams`, `ArrayBuffer`, typed arrays, `Blob`, `FormData` and `ReadableStream<Uint8Array>`.

Ink prepares the body before sending it, so stream uploads start only after the stream finishes. Bodies supplied from JavaScript have a 64 MiB limit. Text, byte and Blob bodies can be sent again after redirects; consumed stream bodies cannot.

For a multipart upload, supply `photoBlob`, `uploadUrl` and an optional cancellation `signal`:

```ts
const form = new FormData();
form.append("caption", "Station entrance");
form.append("photo", photoBlob, "entrance.jpg");
await fetch(uploadUrl, { method: "POST", body: form, signal });
```

Let `fetch` set the multipart `Content-Type` and boundary.

### Upload managed files

Read an attachment with `fetch(file.src)` and use its `blob()` in the upload. Blob slices, File objects and FormData preserve references to native file data. Ink copies that data into a temporary upload file in 32 KiB chunks, checking cancellation between chunks. Attachment bytes stay outside JavaScript memory, including when mixed with text fields.

These uploads are limited by available storage; the 64 MiB JavaScript body limit does not apply to native file data. Managed sources do not allow arbitrary filesystem access. The earlier `fetch(photo.file.uri)` camera path remains supported.

### Transfer limits

Each runtime allows 16 open responses and eight pending uploads. Inactive response streams and temporary upload files expire after 60 seconds without reads or writes. Use [Downloads](/downloads) for transfers that need to survive closing a screen.

## HTTPS and authentication

Release builds require HTTPS/WSS. Debug builds allow local loopback HTTP/WS.

Redirects to another origin remove credentials. Requests do not share browser cookies or login sessions, and there is no browser origin sandbox. If a provider needs cookies, its integration must manage them separately for each account.

## Live connections

`WebSocket` supports WSS, subprotocols, text and binary messages, `binaryType`, and open, message, error and close events. Idle sockets wait for native events without polling.

```ts
const socket = new WebSocket("wss://api.example.com/events");
socket.addEventListener("open", () => socket.send("subscribe"));
socket.addEventListener("message", event => handleMessage(event.data));
// When the connection is no longer needed:
socket.close(1000, "Finished");
```

Supply `handleMessage` to process messages. Open sockets outside rendering and share authentication and reconnection logic across screens.

| Limit | Maximum |
| --- | --- |
| Sockets per runtime | 8 |
| Message size | 256 KiB |
| Incoming or outgoing queue | 512 KiB |

An overflowing incoming queue closes the connection. `bufferedAmount` includes JavaScript sends and the latest native queue reading.

Track message sequence numbers or provider cursors to detect missing data and recover it after reconnecting. Limit queued messages. Sockets are not reliable background delivery; use [background work](/background) and push where available.

## Recovery

Retry failed reads with increasing delays and a retry limit. A server may accept a write even if the response never arrives. Give writes stable operation IDs so the server can recognise retries, or check whether a write succeeded before repeating it.

Save small results with [Store](/store) for offline use. A resource’s in-memory cache does not survive app restarts. Clear private saved content when signing out.

Use [Auth](/auth) for sign-in. npm HTTP clients must work without browser or Node.js APIs.

## Template server

The template’s Network Features screen uses a local server for streams, multipart uploads, WebSocket echo and cancellation:

```sh
bun examples/light-template/scripts/network-server.mjs
adb -s emulator-5554 reverse tcp:18081 tcp:18081
```

Open **Modules → Network → Network Features** in a debug build.

Public WSS and transport limits still need broader verification.
