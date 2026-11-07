---
title: "Android packaging measurements"
description: "Release size and compatibility evidence for the November 2026 packaging changes."
---

Measured on 7 November 2026, starting from SDK commit `037c1e3b`. All sizes below
are bytes. APK sizes are signed file lengths. Dex and native sizes are the sums
of uncompressed ZIP entries. Native libraries remain uncompressed and aligned
for Android loading.

## Release measurements

| App and component | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| Inkify APK | 10,043,824 | 8,755,513 | 1,288,311 (12.8%) |
| Inkify native libraries | 7,339,248 | 6,372,824 | 966,424 (13.2%) |
| Inkify dex | 2,158,984 | 1,896,808 | 262,176 (12.1%) |
| Light-template APK | 34,288,557 | 34,091,957 | 196,600 (0.6%) |
| Light-template native libraries | 15,165,136 | 14,957,512 | 207,624 (1.4%) |
| Light-template dex | 3,382,100 | 3,382,100 | 0 |

Inkify previously contained `libink_android.so` (3,382,328 bytes) and
`libinkify_spotify.so` (3,956,920 bytes). It now contains one
`libink_android.so` (6,372,824 bytes). All 60 JNI exports, including the eight
Spotify entry points, match the baseline. The final library's SHA-256 is
`2ac57f0163a7d707b446017d648e61d2b1dcedb0f99fbb0e2e39ff354b7963a6`.
Building light-template between two Inkify builds leaves this hash unchanged.

The native change includes one final link, dependency and feature unification,
and the app's release profile. Inkify retains its `opt-level = "s"`, thin LTO,
one code generation unit, aborting panics and symbol stripping. These figures do
not isolate deduplication from profile and dependency effects. Incompatible JNI
versions 0.21 and 0.22 remain inside the combined library.

Light-template still uses WebSockets and maps, so it retains OkHttp. Its Ink
library falls from 3,536,176 to 3,328,552 bytes with the wrapper build. MapLibre
and CameraX native libraries are unchanged. Inkify uses neither WebSockets nor
maps; its release shrinker mapping contains no OkHttp or Okio classes.

## Build method

The reference Inkify repository was read without modifications. A separate copy
excluded `node_modules`, `.ink`, `dist` and `bun.lock`. Its Ink dependencies and
npm overrides used this worktree's `file:` packages. After `bun install`, those
installed packages were replaced with symlinks to the selected SDK packages.

Both copies used the worktree's `scripts/ink build`, a throwaway RSA signing key,
`ANDROID_HOME=$HOME/Library/Android/sdk`, Rust 1.96.0, NDK 29.0.14206865, API 34
native linking, the arm64 ABI, release bytecode and release shrinking. No Media3,
ExoPlayer, audio or rendering source changed.

Inkify's migration changes its Rust output to an `rlib`, removes its separate
Cargo/`jniLibs` Gradle task, and changes its Kotlin loader to `ink_android`.
The app's Cargo patches and release profile remain in effect. Ink now builds its
host as an `rlib` and generates a separate final shared-library target for each
app and native configuration. This prevents cached outputs from another app or
capability set from introducing unused code.

Light-template's existing release verifier also required explicit declarations
for MapLibre's `ACCESS_WIFI_STATE` permission and UnifiedPush's exported
`RaiseToForegroundService` and `MessagingReceiverImpl`. These declarations were
added only to the temporary copy's native manifest and used for both measurements.
They did not change the merged APK manifest. The SDK verifier was unchanged.

## Transport inventory

The starting checkout already used Android's
[HttpEngine](https://developer.android.com/reference/android/net/http/HttpEngine)
for fetch, uploads and images. Its only direct application OkHttp consumer was
WebSockets. The packaging change separates that adapter and its dependency into
the `web-sockets` capability.

| Feature | Transport and capability | Preserved contract |
| --- | --- | --- |
| Fetch | HttpEngine; `network` | Resolves at headers; binary response bodies are pulled in 32 KiB chunks. Buffered reads have a 16 MiB limit. HTTP errors return responses; network failures reject. |
| Uploads | HttpEngine file upload provider; `network` | Text, bytes, Blob, URLSearchParams, multipart and stream bodies; JS bodies are staged with a 64 MiB limit. Native-file parts keep bounded file-range copies and avoid that JS limit. |
| Redirects and headers | JS fetch policy over native redirect responses | Follow, manual and error modes; at most 20 follows; 301/302/303 method/body rewriting; 307/308 replay where supported; credentials removed across origins. Header validation and separate response cookies remain. |
| Cancellation and timeouts | AbortSignal and native request ownership | Abort before dispatch, during upload and while reading; 30-second default native-operation timeout; AbortSignal timeouts; 60-second idle expiry; 16 open responses and eight pending uploads per runtime. |
| Cookies | Explicit request and response headers | No shared or persistent browser cookie jar. Server cookies do not become credentials on later requests. |
| Images | HttpEngine; image use includes `network` | 16 MiB response limit, five redirects, image accept header, system HTTP cache and existing cleartext policy. |
| WebSockets | OkHttp 5.4; `web-sockets` depends on `network` | WSS and permitted loopback WS, protocols, text/binary messages, binaryType, close events, 30-second connect timeout, 20-second pings, eight sockets, 256 KiB messages and 512 KiB queues. |
| Downloads | Android HttpURLConnection; `downloads` includes files and network | Durable jobs, two transfers, validated ranges and validators, restart/resume, cancellation, network constraints, three I/O retries, 15-second connect and 30-second read timeouts, and native media sources. |
| Audio | Existing Media3 DefaultHttpDataSource and native media sources; audio capabilities | Existing redirects, playback, downloads integration and native clocks. No data source or playback behaviour changed. |
| Maps | MapLibre SDK; `maps` | MapLibre retains its transitive OkHttp 4.12 dependency. Its SDK and HTTP implementation are unchanged. |
| LightOS push | Android HttpURLConnection and UnifiedPush; `light-sdk-push` | Existing authenticated JSON requests, method-specific timeouts, response limits and redirect policy. |
| Authentication | Android HttpURLConnection; existing auth adapter | Existing form posts, cancellation, HTTPS policy, disabled redirects, 15-second connect and 20-second read timeouts, and 1 MiB response limit. |
| Wallet pass URLs | Android HttpURLConnection; `files` | Existing URL validation, redirects, cancellation checks, 10-second timeouts and 10 MiB pass limit. |
| Connectivity | Android ConnectivityManager; `connectivity` | Native connection-state observation, without an HTTP client dependency. |

WebSocket references in surviving app and dependency modules retain their native
transport. Dynamic `globalThis` lookups also retain it conservatively. Native
adapters or other runtime indirection can declare `capabilities = ["web-sockets"]`.
An omitted socket transport returns an actionable native error. No supported HTTP
body path was rewritten to obtain the size saving.

## IDNA experiment

The final wrapper build was measured with the existing ICU backend and with
`idna_adapter = "=1.1.0"`, which selects the unicode-rs backend.

| Light-template component | ICU baseline | unicode-rs trial | Increase |
| --- | ---: | ---: | ---: |
| Ink native library | 3,328,552 | 3,437,576 | 109,024 |
| APK | 34,091,957 | 34,190,261 | 98,304 |
| Dex | 3,382,100 | 3,382,100 | 0 |

All 1,014 frozen URL results and the existing targeted IDNA cases were unchanged
in both split and inline QuickJS runtimes. The trial increased native size, so
it was reverted. The final SDK keeps `idna_adapter` 1.2.2 and ICU data 2.4, and the
combined Inkify graph resolves the same Unicode data.

The checkout did not contain a checked-in 986-case WPT comparison. The added
reproducible corpus contains 899 WPT parsing cases and 115 host/hostname setters,
with independent expected results from the published `whatwg-url` 14.2.0/tr46
parser. The fixture generator pins WPT revision
`8c17bfeb3e66a2986be16b0e004f85ace68db647`. This establishes compatibility with the
previous parser for those inputs, rather than claiming complete WPT conformance.

## Validation

- Three CLI contracts verify migration guidance and distinct, stable native
  outputs across apps and capability configurations.
- Forty-six compiler and network-data contracts pass, including WebSocket
  capability selection, headers, body types, multipart framing and parsing.
- Three web-runtime contracts pass, including all 1,014 URL comparisons in both
  split and inline runtime builds.
- Eleven Android adapter contracts pass, including real HTTP bytes, managed-file
  uploads, cancellation, limits, ownership and the real 60-second idle expiry.
- Twenty-one full Android app contracts pass through JS, QuickJS, Rust/JNI and
  production Kotlin. They include one packaged library, an app JNI call, fetch,
  streams, uploads, redirects, cookies, timeouts, downloads, managed-file multipart
  slices and WebSocket protocol, text, binary and clean-close behaviour.
- The API 34 arm64 `Light_Phone_III` emulator ran with `-writable-system`.
  A separate release package, `com.vandam.inkify.size`, exercised all eight Spotify
  JNI entry points using the exact final native library hash. Initialisation and
  session clearing succeeded; invalid arguments and closed handles returned the
  expected native errors. HTTPS fetch returned status 200 and expected content
  with OkHttp absent. The probe package was removed afterwards.
- `cargo check -p ink-cli` and `git diff --check` pass. Strict Clippy encounters
  existing warnings in unchanged `ink-core` code and a `needless_borrow` in
  `development.rs`. The CLI-only check passes with that existing lint allowed.

Spotify authentication, catalogue access, encrypted downloads and actual playback
require credentials and were not exercised. Physical Light Phone performance,
release cleartext policy, external map servers, LightOS push delivery and hardware
media behaviour were not requalified. Existing audio and external SDK transports
remain unchanged. Cargo's compatible version and feature unification can change
shared dependency code; review the generated graph when upgrading app or SDK pins.
