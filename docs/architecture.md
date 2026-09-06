---
title: "How Ink works"
description: "TypeScript app behaviour, QuickJS-ng execution and retained native rendering."
tag: "In development"
---

> **In development.** React, QuickJS-ng, retained rendering, background runtimes, automatically measured lists and native subtree updates are implemented. Focused emulator checks and physical LP3 benchmarks are recorded; real-app workflow validation remains deferred. See [verification](verification-2026-09-06.md).

Every Ink app runs React and JavaScript in QuickJS-ng on the phone. The compiler type-checks and bundles the app, prepares its assets and selects native integrations.

[Product design](product-design.md) defines the intended authoring experience and module scope. This page explains the current engine; historical measurements below are evidence for that architecture rather than promises for every app.

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

[lightos]
enabled = true
```

`package.json` and its lockfile describe JavaScript dependencies. `ink.toml` describes the installed Android app and explicit native integration. Pure JavaScript packages need no Ink-specific registration. The build reads versioned `ink-native.json` requirements from resolved package modules. Explicit app capabilities are additive. See [build contracts](build-contracts.md).

## Build responsibilities

The build resolves normal package exports, removes TypeScript types, compiles JSX, bundles reachable code and includes declared assets. It emits a UI bundle and, when configured, a worker bundle. Release builds also package web runtime code separately for loading on first use; `INK_SPLIT_WEB=0` disables that split for comparisons. General application code splitting and downloading executable code after installation are not supported.

Supported packages declare module requirements. One versioned capability catalogue supplies dependency closure, permissions and Android source groups to the compiler and Gradle. A general third-party native ABI/build extension contract remains separate. Native modules and required capability groups are linked together into the APK. Tree shaking can remove unused JavaScript; it cannot guarantee that one method can be extracted from an indivisible native SDK.

Apps ship bundled JavaScript, prepared icons, assets and native capability metadata. The build currently bundles JavaScript source for both development and release.

## Runtime responsibilities

The foreground app has one long-lived QuickJS-ng runtime on a dedicated JavaScript thread. Ink pumps promise jobs and native completions, hosts timers and networking, and schedules component updates. Native calls return promises instead of blocking that thread on I/O.

Rust owns the retained UI tree, layout, text measurement, hit testing, scrolling, image transforms and rendering. JavaScript supplies application state and component descriptions. Ink batches changes across the native seam and applies consistent updates at frame boundaries. Native scrolling can continue while JavaScript is busy, although new content, commands and UI state will wait for it.

Non-structural React commits patch changed native subtrees. Structural and navigation changes rebuild the tree; layout still recomputes. Lists mount a window of rows based on the native viewport, using cached content measurements and estimates. There is one List interface, without height hints or a separate fixed-height mode. Stable keys preserve the visible anchor; see [list behaviour](lists.md).

A visually idle app requests no rendering frames. This is not a promise of zero CPU usage: application timers, sockets, background work and media can still consume power.

## Ownership

| Owner | Examples | End of lifetime |
| --- | --- | --- |
| Component | Form state, actions, memoised calculations | Unmount |
| Visible screen | Resource observations, camera, foreground location | Hidden screen or backgrounded app |
| App runtime | Account module, shared in-memory store | Process/runtime disposal |
| Native service | Detached audio | Explicit stop or Android termination |
| Durable storage | Store preferences, queued jobs | Explicit deletion or app-data removal |

JavaScript garbage collection is not a lifecycle mechanism for a camera, socket or player. UI hooks and explicit `close()`/unsubscribe operations release native work. Reactivation obtains fresh handles; stale results cannot update a replacement owner. Native handles may be held in memory but are not serialisable. Durable IDs reconnect to stored state instead of reviving an old pointer.

Background jobs start a separate headless runtime with their registered worker entry point. They cannot share foreground globals. Native audio does not need a continuously running JavaScript loop. [Downloads](downloads.md) uses persisted native jobs for HTTP file transfers. Android can stop work; storage and domain reconciliation provide recovery.

## Package execution and trust

JavaScript dependencies execute in the app's runtime and share its available host APIs. A package namespace is not a security sandbox. Native dependencies execute with the app's Android authority. A lockfile and build report make dependencies reproducible and inspectable; they do not make arbitrary third-party code safe.

For hashing, use [`@noble/hashes`](https://github.com/paulmillr/noble-hashes) instead of an Ink crypto package. Secure credentials belong in [Secure store](secure-store.md). JavaScript hashing does not provide Android Keystore-backed keys.

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
