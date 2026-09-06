# JavaScript runtime compatibility

Ink runs QuickJS-ng on a dedicated thread. Installing or bundling a dependency does not establish that its runtime requirements are available.

An app should extend the shipped configuration:

```json
{
  "extends": "ink/tsconfig",
  "include": ["**/*.ts", "**/*.tsx"],
  "exclude": ["node_modules", ".ink"]
}
```

This uses ECMAScript 2022 declarations and `ink/runtime-types`, without TypeScript's DOM library or automatically included Node globals. The runtime declarations derive networking, URL, blob, socket and stream types from the implementations Ink actually installs. They do not claim every browser method on similarly named classes. The compiler loads `ink/runtime` before application and worker code.

The standard ECMAScript declarations are a language baseline, not a promise that every optional engine facility exists. In particular, QuickJS-ng does not supply an internationalisation engine for `Intl` here. TypeScript's standard ECMAScript declarations include `Intl`; use an explicit compatible polyfill if a dependency requires it. The native runtime has a 64 MiB JavaScript heap limit and a 512 KiB stack limit.

## Installed facilities

| Facility | Implementation and limits |
| --- | --- |
| ECMAScript values, promises, collections, typed arrays, WeakRef and FinalizationRegistry | QuickJS-ng. There is no Node module loader or browser document. |
| `setTimeout`, `setInterval`, clearing timers and `queueMicrotask` | Function callbacks, numeric timer identifiers; delays clamped to 32-bit milliseconds. Timers share turns with promise jobs and native messages. |
| `console` | `log`, `info`, `warn`, `error`, `debug`; values are converted to strings and forwarded to Android logging. No browser console inspection methods. |
| `performance` | `now()` and `timeOrigin` only. No marks, measures or observers. |
| `TextEncoder`, `TextDecoder` | UTF-8, incremental decoding, fatal decoding and BOM options; other encodings reject. |
| `Event`, `CustomEvent`, `EventTarget`, `DOMException` | Standalone events; no DOM tree, bubbling path through elements or browser event sources. |
| `AbortController`, `AbortSignal` | Reasons, abort events, `throwIfAborted`, `abort`, `timeout`, `any`. |
| `URL`, `URLSearchParams` | Bundled `whatwg-url`; no object URL registry. |
| `Blob`, `File`, `FormData` | In-memory bytes, streams and multipart data. These do not expose arbitrary filesystem access. |
| Readable, writable and transform streams; queuing strategies | Bundled `web-streams-polyfill`, including backpressure, readers, writers and cancellation. No browser-specific streams such as compression or text-encoder streams. |
| `fetch`, `Headers`, `Request`, `Response` | Ink's native HTTP adapter. HTTPS, managed file reads and development localhost HTTP. Response streaming, abort and redirect handling are implemented. Upload input is consumed into native temporary storage before the HTTP request opens; live producer-to-network upload streaming is not implemented. `credentials` is retained on Request but does not supply a browser cookie jar. |
| `WebSocket`, `MessageEvent`, `CloseEvent` | Native socket transport with text/binary messages, explicit close and buffering limits. No browser origin/document integration. |

No `window`, `document`, `navigator`, DOM nodes, localStorage, IndexedDB, service workers, browser `Worker`, XMLHttpRequest, Web Crypto, `atob`/`btoa`, Node `process`, `Buffer`, `fs`, `net`, or React Native native modules are installed. Ink's background package supplies separate supported job runtimes; it is not a browser Worker implementation. Use native media URIs rather than copying large media through JavaScript byte arrays.

## Dependency patterns and existing demonstrations

| Pattern | Concrete code | What it establishes |
| --- | --- | --- |
| Hooks/context and state | Template appearance/settings providers and screens use React 19 hooks and context. | Ordinary React composition targets Ink's reconciler. Components that assume DOM elements remain incompatible. |
| Pure JavaScript data conversion | `base64-js` is used by Ink's HTTP and WebSocket adapters; `whatwg-url` parses URLs. | Bundled data-only libraries can execute without Node or a browser document. |
| Stream composition | `web-streams-polyfill` underlies blob, multipart, HTTP and socket data handling. | A dependency can provide missing platform facilities when it has a suitable JavaScript implementation. |
| Networking | Template `data/network-features.ts` and its network features screen demonstrate streamed responses, multipart upload, abort, redirects and text/binary WebSocket echo. | These flows are available for explicit manual verification against the included local server. Type-checking alone does not establish live network behaviour. |
| Application state and decoding libraries | Check their resolved JavaScript and declarations for Node, DOM and native requirements, then exercise the operations the app uses. | A package's framework label or successful installation is insufficient evidence; no blanket compatibility guarantee is made. |

The template and framework passed type-checking with `skipLibCheck` disabled after adopting the runtime declarations. Inspecting TypeScript's resolved file list found neither `lib.dom.d.ts` nor `@types/node`. React's declaration package contributes empty element interfaces for JSX compatibility, but does not install browser globals. `whatwg-url` references the standard ES2020 declarations; the stream declarations use their own interfaces and the supplied AbortSignal. Re-run this audit when changing dependencies: a dependency can explicitly reference the DOM library even when the app's `lib` setting excludes it.

Physical-device and wider application workflows remain deferred for verification with the user. See [native lifetimes](runtime-contracts.md) for cancellation, ownership and runtime restart behaviour.
