---
title: "How Ink works"
description: "TypeScript app behaviour, QuickJS-ng execution and retained native rendering."
tag: "Design specification"
---

App logic executes as JavaScript on the phone. The compiler bundles code and connects JSX to Ink; it does not translate arbitrary application logic into Rust.

```text
App.tsx + TypeScript + npm dependencies
                    ↓
       JavaScript bundle + assets
                    ↓
          QuickJS-ng + React
                    ↕
    batched UI changes and native calls
                    ↕
       Rust layout and retained UI
                    ↓
             Vulkan renderer

Android / LightOS ↔ native packages ↔ JavaScript
```

## A small app project

```toml
# ink.toml
name = "Weather"
package = "com.example.weather"
version = "1.0.0"
version_code = 1

[android]
modules = ["@ink/location"]

[lightos]
enabled = true
```

`package.json` and its lockfile describe JavaScript dependencies. `ink.toml` describes the installed Android app and explicit native integration. Pure JavaScript packages need no Ink-specific registration. Native packages are installed through npm and registered by `ink add`; see [Packages](modules.md).

## Build responsibilities

The build resolves normal package exports, removes TypeScript types, compiles JSX, bundles reachable code and includes declared assets. Literal dynamic imports can become bundled chunks; they are not a way to download executable code after installation.

Native package manifests describe entry points, ABI compatibility, permissions, Android components and build dependencies. Native modules and required capability groups are linked together into the APK. Tree shaking can remove unused JavaScript; it cannot guarantee that one method can be extracted from an indivisible native SDK.

The runtime and bundle formats are internal. Release builds may use engine-version-matched bytecode where supported, but applications never store bytecode as user data or depend on an engine-specific interface.

## Runtime responsibilities

The foreground app has one long-lived QuickJS-ng runtime on a dedicated JavaScript thread. Ink pumps promise jobs and native completions, hosts timers and networking, and schedules component updates. Native calls return promises instead of blocking that thread on I/O.

Rust owns the retained UI tree, layout, text measurement, hit testing, scrolling, image transforms and rendering. JavaScript supplies application state and component descriptions. Ink batches changes across the native seam and applies consistent updates at frame boundaries. Native scrolling can continue while JavaScript is busy, although new content, commands and UI state will wait for it.

A visually idle app requests no rendering frames. This is not a promise of zero CPU usage: application timers, sockets, background work and media can still consume power.

## Ownership

| Owner | Examples | End of lifetime |
| --- | --- | --- |
| Component | Form state, actions, memoised calculations | Unmount |
| Visible screen | Resource observations, camera, foreground location | Hidden screen or backgrounded app |
| App runtime | Account module, shared in-memory store | Process/runtime disposal |
| Native service | Detached audio, managed downloads | Explicit stop or Android termination |
| Durable storage | Preferences, records, files, queued jobs | Explicit deletion or app-data removal |

JavaScript garbage collection is not a lifecycle mechanism for a camera, socket or player. UI hooks and explicit `close()`/unsubscribe operations release native work. Reactivation obtains fresh handles; stale results cannot update a replacement owner. Native handles may be held in memory but are not serialisable. Durable IDs reconnect to stored state instead of reviving an old pointer.

Background jobs start a separate headless runtime with their registered worker entry point. They cannot share foreground globals. Native audio and download services do not need a continuously running JavaScript loop. Android can stop work; storage and domain reconciliation provide recovery.

## Package execution and trust

JavaScript dependencies execute in the app's runtime and share its available host APIs. A package namespace is not a security sandbox. Native dependencies execute with the app's Android authority. A lockfile and build report make dependencies reproducible and inspectable; they do not make arbitrary third-party code safe.

## LightOS integration

Ink adapts to available LightOS services, preferences and Android lifecycle. Host integration and distribution eligibility are separate contracts; see [LightOS](light-sdk.md).

## Performance

Keep gestures and bulk media processing native. Cache native resources, batch updates, page large collections and publish progress at useful rates. A faster JavaScript engine cannot compensate for decoding full-resolution images in JS, repeatedly copying message histories or running unnecessary polling loops.

### LP3 runtime benchmark

Measured on a physical Light Phone III running Android 14 on 4 September 2026. All variants use Ink's native renderer; the JavaScript variants replace the counter's increment arithmetic with a synchronous engine call.

| Metric | Native Ink | QuickJS-ng | Hermes source | Hermes bytecode |
| --- | ---: | ---: | ---: | ---: |
| APK size | 3.01 MB | 3.70 MB | 7.19 MB | 7.19 MB |
| Idle memory, median PSS | 23.88 MiB | 24.54 MiB | 27.12 MiB | 26.24 MiB |
| Cold start, median | 316 ms | 313 ms | 323 ms | 327 ms |
| State/UI update, median | 0.274 ms | 0.301 ms | 0.302 ms | 0.301 ms |

QuickJS-ng added 0.69 MB to the native APK and 0.66 MiB of idle PSS in this run, with a similar state/UI update time to both Hermes variants. This supports its use for a small native app with lightweight JavaScript actions.

The harness interleaved variants across 15 cold starts, five idle-memory samples and five sets of 100 taps per variant. All 2,000 counter updates were validated. QuickJS-ng was version 0.15.1 through rquickjs 0.12.2; Hermes used the stock Android 250829098.0.17 release with its required support libraries. A different Hermes build could change its footprint.

These measurements cover the synchronous counter embedding, which calls JavaScript on the UI thread. They do not measure the React authoring layer, the dedicated JavaScript thread described above, asynchronous host APIs or battery life. Startup distributions overlap; the small median differences do not establish a speed advantage. State/UI update time excludes display latency and is not an input-to-photon measurement.
