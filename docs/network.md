---
title: "Network"
description: "Use fetch, streams and sockets with native transport."
tag: "Design specification"
---

Use standard `fetch` for HTTP. Enable the `network` capability in `ink.toml`; see the [JavaScript environment](runtime.md). Ink runs transport natively and delivers completions to JavaScript.

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

This fragment assumes your provider's `decodeDepartures` function. A successful HTTP response still needs decoding. `fetch` rejects transport failures and cancellation; HTTP error statuses remain responses. Apply timeouts with `AbortSignal.timeout()` and propagate caller cancellation with `AbortSignal.any()`.

## Requests and streams

The target profile includes headers, redirects, streamed request/response bodies and multipart uploads. Use managed files for large attachments. A convenience `json()` call materialises the complete body; stream or download large content instead. Backpressure bounds native-to-JavaScript queues, but collecting all chunks in an array defeats it.

Use HTTPS. Redirect handling strips credentials when crossing origins. There is no browser origin sandbox or ambient browser login session. A provider that needs cookies must use an explicit account-scoped cookie jar supplied by its adapter; ordinary fetch does not borrow browser cookies.

## Live connections

`WebSocket` provides the standard connection interface. A domain module owns connection state, authentication, reconnect delays and reconciliation. A screen observes that module; rendering must not open a socket.

For ordered message streams, track provider cursors or sequence numbers and recover after gaps. Bound pending sends and incoming queues. A socket surviving briefly after backgrounding is not a delivery guarantee; use [background work](background.md) and push where available.

## Recovery

Retry selected reads with bounded backoff. Writes need an idempotency key or a way to reconcile uncertain outcomes. Going offline can leave a request accepted remotely even if no response arrives locally.

Persist useful results explicitly with [Records](records.md). The HTTP cache, an in-memory UI resource and your offline database serve different purposes. Account sign-out must remove account-specific persisted content according to the app's policy.

Use [Downloads](downloads.md) for durable transfers and [Auth](auth.md) for account flows. An npm HTTP client is welcome if its requirements match the host profile.
