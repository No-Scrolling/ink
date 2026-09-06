---
title: "Network"
description: "Use fetch with native HTTP transport."
---

Ink supports fetch, response streams, Blob, File, FormData, multipart uploads and WebSocket. Public WSS and transport limits still need broader verification.

Use standard `fetch` for HTTP. Enable the `network` capability in `ink.toml`. Ink runs transport natively and delivers completions to JavaScript.

```toml
capabilities = ["network"]
```

```ts
export async function getDepartureBoard(stopId: string, signal?: AbortSignal) {
  const url = new URL("https://api.example.com/departures");
  url.searchParams.set("stop", stopId);
  const response = await fetch(url, { signal });
  if (!response.ok) throw new Error(`Departures unavailable (${response.status})`);
  const body: unknown = await response.json();
  return decodeDepartures(body);
}
```

`decodeDepartures` validates the response for your app. A successful HTTP response still needs decoding. `fetch` rejects transport failures and cancellation; HTTP error statuses remain responses. Apply timeouts with `AbortSignal.timeout()` and propagate caller cancellation with `AbortSignal.any()`.

## Requests

Fetch resolves when response headers arrive. Read `response.body` incrementally with a reader, async iterator or stream pipeline. Native transport reads one 32 KiB chunk on demand rather than buffering the whole response. Cancel the reader or abort the request when you no longer need it.

For HTTP responses, `json()`, `text()`, `blob()`, `formData()` and `arrayBuffer()` read the whole body into memory, with a 16 MiB limit. Stream larger responses instead. Managed local file responses preserve native storage when `blob()` is called; explicit text and byte reads still read their data into memory. A response body can be consumed once; `clone()` creates a second branch. Consuming only one stream clone can buffer data for the other branch, so avoid cloning large streams.

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

Request bodies accept text, URLSearchParams, ArrayBuffer, typed arrays, Blob, FormData and ReadableStream of Uint8Array. Uploads are staged natively in bounded chunks before transmission, with a 64 MiB limit. This keeps individual bridge messages small and supports rewinding native uploads. Streaming request bodies are therefore accepted but do not begin transmission until their stream completes. Text, byte and Blob bodies can be replayed for redirects; a consumed stream body cannot.

```ts
const form = new FormData();
form.append("caption", "Station entrance");
form.append("photo", photoBlob, "entrance.jpg");
await fetch(uploadUrl, { method: "POST", body: form, signal });
```

Let fetch set the multipart Content-Type, including its boundary. [Managed attachments](/files), including accepted camera photos, can be read with `fetch(file.src)`. Calling `blob()` preserves native file ranges, including through Blob slicing, File construction and FormData. Native code copies those ranges into the upload spool without bringing attachment bytes into JavaScript; its size is constrained by available storage. The 64 MiB limit applies to bodies streamed from JavaScript. Managed sources do not grant arbitrary filesystem access. The earlier `fetch(photo.file.uri)` camera path remains supported for compatibility.

Each runtime allows 16 open responses and eight pending uploads. An abandoned response stream or upload spool expires after 60 seconds without reads or writes. Attachment preparation uses bounded 32 KiB operations and checks cancellation between chunks; file bytes remain native even when mixed with large text fields. Use [Downloads](/downloads) for durable incoming transfers.

Use HTTPS. Redirect handling strips credentials when crossing origins. There is no browser origin sandbox or ambient browser login session. A provider that needs cookies must use an explicit account-scoped cookie jar supplied by its adapter; ordinary fetch does not borrow browser cookies.

## Live connections

`WebSocket` supports WSS, subprotocols, text and binary messages, `binaryType`, open/message/error/close events and graceful closure. Transport runs natively through OkHttp. Incoming events wait in native code until JavaScript reads them; idle sockets do not busy-poll.

```ts
const socket = new WebSocket("wss://api.example.com/events");
socket.addEventListener("open", () => socket.send("subscribe"));
socket.addEventListener("message", event => handleMessage(event.data));
// When the connection is no longer needed:
socket.close(1000, "Finished");
```

There are at most eight sockets per runtime. Messages are limited to 256 KiB and incoming/outgoing queues to 512 KiB; overflowing an incoming queue closes the connection. `bufferedAmount` includes JavaScript sends and the latest native queue snapshot. Manage authentication and reconnection in a shared module. Do not open sockets during rendering.

For ordered message streams, track provider cursors or sequence numbers and recover after gaps. Bound pending sends and incoming queues. A socket surviving briefly after backgrounding is not a delivery guarantee; use [background work](/background) and push where available.

## Recovery

Retry selected reads with bounded backoff. Writes need an idempotency key or a way to reconcile uncertain outcomes. Going offline can leave a request accepted remotely even if no response arrives locally.

Persist small results explicitly with [Store](/store); its read-only SQLite interface queries imported database assets. The HTTP cache, an in-memory UI resource and your offline database serve different purposes. Account sign-out must remove account-specific persisted content according to the app's policy.

Use [Downloads](/downloads) for durable transfers and [Auth](/auth) for account flows. You can use an npm HTTP client if it supports [Ink’s runtime](/runtime-compatibility).

## Template server

The template’s Network Features screen exercises streams, multipart uploads, WebSocket echo and cancellation against the included server:

```sh
bun examples/light-template/scripts/network-server.mjs
adb -s emulator-5554 reverse tcp:18081 tcp:18081
```

Open Modules → Network → Network Features. Debug builds accept cleartext HTTP/WS on loopback for this local server; release transport requires HTTPS/WSS.
