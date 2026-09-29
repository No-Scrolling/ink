---
title: "Network"
description: "Use fetch with native HTTP transport."
---

Use `@ink/network` for `fetch`, response streams, `Blob`, `File`, `FormData` and `WebSocket`.

Import it in each module that uses these APIs.

For services that require unencrypted HTTP, opt in explicitly in `ink.toml`:

```toml
capabilities = ["network-cleartext"]
```

This includes `network` and permits cleartext traffic throughout the app, including
podcast feeds, downloads and audio playback. Audio can also follow redirects between
HTTP and HTTPS. Without this capability, remote audio requires HTTPS and cleartext
network traffic remains restricted to local development hosts.

## Make a request

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

`decodeDepartures` validates the response for your app.

`fetch` rejects network failures and cancellation. HTTP errors such as 404 still return a response; check `response.ok` before reading it.

### Cancel or time out

Use `AbortSignal.timeout()` for timeouts and `AbortSignal.any()` to combine cancellation signals.

## Read a response

`fetch` resolves when headers arrive. A response body can be read once.

### Read the whole body

`json()`, `text()`, `blob()`, `formData()` and `arrayBuffer()` read an entire HTTP response into memory, up to 16 MiB.

### Stream a response

Body bytes cross the native bridge as typed byte arrays. HTTP, managed-file reads and WebSocket binary messages no longer encode those bytes as base64 inside JSON. This preserves the public APIs and their existing limits; it still copies bytes across runtime and Android ownership boundaries.

Read larger bodies with a reader, async iterator or stream pipeline. Ink reads 32 KiB chunks as needed:

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

Supply `url`, a cancellation `signal` and a `processChunk` handler. Cancel the reader or abort the request when finished.

`clone()` creates a second stream. Unread data can accumulate in memory, so avoid cloning large responses.

### Read a local file

Local [managed files](/files) behave differently: `blob()` keeps their data in native storage. `text()` and `arrayBuffer()` still load it into JavaScript memory.

## Upload data

Request bodies accept text, `URLSearchParams`, `ArrayBuffer`, typed arrays, `Blob`, `FormData` and `ReadableStream<Uint8Array>`.

Bodies supplied from JavaScript have a 64 MiB limit. Ink prepares the body before sending it, so stream uploads start only after the stream finishes.

Text, byte and Blob bodies can be sent again after redirects; consumed stream bodies cannot.

### Multipart uploads

For a multipart upload, supply `photoBlob`, `uploadUrl` and an optional cancellation `signal`:

```ts
const form = new FormData();
form.append("caption", "Station entrance");
form.append("photo", photoBlob, "entrance.jpg");
await fetch(uploadUrl, { method: "POST", body: form, signal });
```

Let `fetch` set the multipart `Content-Type` and boundary.

### Upload managed files

Read an attachment with `fetch(file.src)` and use its `blob()` in the upload. Blob slices, File objects and FormData retain references to native file data, without loading it into JavaScript memory.

Ink sends one native copy request per managed file range. Android copies into the temporary upload file on its network executor with a bounded 64 KiB buffer and checks cancellation between reads. JavaScript stream bodies still send 32 KiB chunks. Native file data is limited by available storage, not the 64 MiB JavaScript body limit.

Managed sources do not allow arbitrary filesystem access. The earlier `fetch(photo.file.uri)` camera path remains supported.

## Transfer limits

| Resource | Limit |
| --- | --- |
| Open responses per runtime | 16 |
| Pending uploads per runtime | 8 |
| Inactive response streams and upload files | Expire after 60 seconds without reads or writes. |

Use [Downloads](/downloads) for transfers that must survive closing a screen.

## HTTPS and authentication

Release builds require HTTPS/WSS. Debug builds allow local loopback HTTP/WS.

Redirects to another origin remove credentials. Requests do not share browser cookies or login sessions. Manage provider cookies separately for each account.

There is no browser origin sandbox. npm HTTP clients must work without browser or Node.js APIs. Use [Auth](/auth) for sign-in.

## WebSocket connections

`WebSocket` supports WSS, subprotocols, text and binary messages, `binaryType`, and open, message, error and close events. Idle sockets wait for native events without polling.

Unencrypted `ws://` is restricted to loopback hosts (`localhost`, `127.0.0.1`, `::1`) and requires a debug build or the `network-cleartext` capability. Remote sockets require WSS.

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

### Reconnect and recover messages

Track message sequence numbers or provider cursors to detect missing data and recover it after reconnecting. Limit queued messages. Sockets are not reliable background delivery; use [background work](/background) and push where available.

## Retry failed requests

Retry failed reads with increasing delays and a retry limit. A server may accept a write even if the response never arrives. Give writes stable operation IDs so the server can recognise retries, or check whether a write succeeded before repeating it.

### Save offline data

Save small results with [Store](/store). A resource’s in-memory cache does not survive app restarts. Clear private saved content when signing out.

## Connection state and downloads

Separate imports provide [connection state](/connectivity) and [managed downloads](/downloads):

```ts
import { connectivity } from "@ink/network/connectivity";
import { downloads } from "@ink/network/downloads";
```

These imports work independently and do not install web globals. `@ink/network` alone includes neither feature. Images and maps load without it.
