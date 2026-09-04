---
title: "JavaScript environment"
description: "Know which JavaScript packages and host APIs an Ink app can use."
tag: "Design specification"
---

Ink bundles TypeScript into JavaScript and executes it in QuickJS-ng. The engine implements JavaScript; Ink supplies the surrounding phone environment. An npm package is compatible when its runtime requirements fit that environment.

## Target host profile

| Available interface | Contract |
| --- | --- |
| Objects, arrays, collections, promises, generators, typed arrays | JavaScript language facilities; no Ink expression subset. |
| `URL`, `URLSearchParams`, `TextEncoder`, `TextDecoder` | Standard URL and UTF-8 handling. |
| `fetch`, `Headers`, `Request`, `Response` | Native-backed networking; requires the network capability. |
| `AbortController`, `AbortSignal` | Cancellation, including timeout signals. |
| Timers, `queueMicrotask`, `performance.now()` | Hosted by Ink's event loop; no durable scheduling guarantee. |
| `WebSocket` | Native-backed connections through the network capability. |
| `ReadableStream`, `WritableStream`, `TransformStream` | Bounded streaming and backpressure; binary chunks use typed arrays. |
| `Blob`, `FormData` | HTTP payloads; managed files can be uploaded without copying whole files into JS. |
| `crypto.getRandomValues`, `crypto.randomUUID` | Native secure randomness. |
| `console` | Development logs; application code remains responsible for what it prints. |

Browser layout, DOM, Canvas, WebGL, Web Audio and browser storage APIs are not provided. There is no Node `process`, filesystem, `Buffer`, `require` at runtime, native Node addon ABI or arbitrary Android reflection. Bundle supported CommonJS dependencies at build time. Use `Uint8Array`, [Files](files.md), [Store](store.md) and Ink's native views instead.

Do not assume every `Intl` locale, timezone or Web Crypto algorithm exists. The shipped host profile lists those supported by that release; unsupported functionality fails explicitly. Dependencies must not silently substitute weak randomness or incorrect date handling.

## Networking capability

```toml
[android]
capabilities = ["network"]
```

The global names are stable whether or not networking is linked. Without the capability, network operations reject with `capability-not-linked`. Native packages can declare this requirement in their manifests. `ink info` reports why each capability is present; `ink check` catches known undeclared use but cannot prove all dynamic paths.

## Package selection

A parser, date utility, schema validator or protocol implementation that uses supported JavaScript can usually be bundled directly. A browser SDK might need an injected transport or storage adapter. A React Native wrapper needs a real Ink-native counterpart; installing it does not bring its renderer or native registration with it.

Prefer ESM entry points and explicit subpath imports. Literal `import("./details")` is bundled; a server-provided module URL is not. Configure public build-time values through `ink.config` imports generated from the app config; do not assume `process.env` exists. Values bundled into an APK are public, including API keys embedded in source.

## Async execution

Promise continuations and native callbacks execute on the JavaScript thread. `await` releases that thread while an operation is pending; it does not move CPU-heavy JavaScript elsewhere. Long loops can delay app updates even though native scrolling keeps moving.

The foreground runtime is suspended or disposed according to Android lifecycle. Timers are suitable for a visible countdown or debounce, not an alarm, download engine or background scheduler. [Background jobs](background.md) receive a fresh headless runtime and durable inputs.

Use bounded native operations for decoding media, database queries, cryptography and high-rate sensor processing. There is no promise that adding more JavaScript workers makes a small phone more efficient.

## Diagnosing compatibility

`ink check` reports unresolved imports and known unsupported built-ins. `ink logs` shows missing-host-API failures and source-mapped exceptions. Exercise actual package paths on the phone: successfully bundling a library does not demonstrate that its authentication, streaming or storage paths work.
