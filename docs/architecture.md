---
title: "How Ink works"
description: "TypeScript app behaviour, QuickJS-ng execution and retained native rendering."
---

Ink runs React in QuickJS-ng, with native layout and Vulkan rendering. The compiler checks TypeScript, bundles JavaScript, prepares assets and selects native integrations. For app development, start with [Create an app](/standalone).

See [product design](/product-design) for scope.

```text
app/ pages + TypeScript + npm dependencies
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

`package.json` and its lockfile describe JavaScript dependencies. `ink.toml` describes the installed Android app and explicit native integration. Pure JavaScript packages need no Ink-specific registration. The build reads versioned `ink-native.json` requirements from resolved package modules. Explicit app capabilities are additive. See [build contracts](/build-contracts).

## Build responsibilities

The build resolves normal package exports, removes TypeScript types, compiles JSX, bundles reachable code and includes declared assets. It emits a UI bundle and, when configured, a worker bundle. Release builds also package web runtime code separately for loading on first use; `INK_SPLIT_WEB=0` disables that split for comparisons. General application code splitting and downloading executable code after installation are not supported.

Supported packages declare module requirements. One versioned capability catalogue supplies dependency closure, permissions and Android source groups to the compiler and Gradle. A general third-party native ABI/build extension contract remains separate. Native modules and required capability groups are linked together into the APK. Tree shaking can remove unused JavaScript; it cannot guarantee that one method can be extracted from an indivisible native SDK.

Apps ship bundled JavaScript, prepared icons, assets and native capability metadata. The build currently bundles JavaScript source for both development and release.

## Runtime responsibilities

The foreground app has one long-lived QuickJS-ng runtime on a dedicated JavaScript thread. Ink pumps promise jobs and native completions, hosts timers and networking, and schedules component updates. Native calls return promises instead of blocking that thread on I/O.

Rust owns the retained UI tree, layout, text measurement, hit testing, scrolling, image transforms and rendering. JavaScript supplies application state and component descriptions. Ink batches and applies completed React updates before rendering. When idle, it can present an update without waiting for a new display callback. Gestures and ongoing frames use Android's frame scheduler. Native scrolling can continue while JavaScript is busy, although new content, commands and UI state will wait for it.

Non-structural React commits patch changed native subtrees. Structural and navigation changes rebuild the tree; layout still recomputes. Lists mount a window of rows based on the native viewport, using cached content measurements and estimates. There is one List interface, without height hints or a separate fixed-height mode. Stable keys preserve the visible anchor; see [list behaviour](/lists#collections).

The renderer reuses prepared geometry between changes, including text that moves without changing its appearance or clipping. It combines adjacent draws of the same image while preserving image order.

The text and icon atlas starts at 1 MiB. When full, it clears cached entries and rebuilds the current scene, including texture coordinates. It grows to 4 MiB, then 16 MiB only if that scene needs more space. Scenes exceeding the limit report a rendering error. Images use a separate cache.

A visually idle app requests no rendering frames. This is not a promise of zero CPU usage: application timers, sockets, background work and media can still consume power.

## Ownership

| Owner | Examples | End of lifetime |
| --- | --- | --- |
| Component | Form state, actions, memoised calculations | Unmount |
| Visible React screen | Resource observations and hook effects | Screen hidden or removed |
| Foreground native controller | Camera, microphone, foreground location | Native controller lifecycle, including app backgrounding |
| App runtime | Account module, shared in-memory store | Process/runtime disposal |
| Native service | Detached audio | Explicit stop or Android termination |
| Durable storage | Store preferences, queued jobs | Explicit deletion or app-data removal |

Hiding a route is different from putting the app in the background. Resource polling follows React subscriptions; app backgrounding alone does not unsubscribe them. Native controllers apply their own Android lifecycle rules.

Hooks release native resources when their effects end. For explicit handles, call `close()` or unsubscribe. Garbage collection does not close a camera, socket or player. Reconnecting creates fresh handles; use saved IDs, not handles, to reopen durable resources.

Background jobs start a separate headless runtime with their registered worker entry point. They cannot share foreground globals. Native audio does not need a continuously running JavaScript loop. [Downloads](/downloads) uses persisted native jobs for HTTP file transfers. Android can stop work; storage and domain reconciliation provide recovery.

## Package execution and trust

JavaScript dependencies execute in the app's runtime and share its available host APIs. A package namespace is not a security sandbox. Native dependencies execute with the app's Android authority. A lockfile and build report make dependencies reproducible and inspectable; they do not make arbitrary third-party code safe.

For hashing, use [`@noble/hashes`](https://github.com/paulmillr/noble-hashes) instead of an Ink crypto package. Secure credentials belong in [Secure store](/secure-store). JavaScript hashing does not provide Android Keystore-backed keys.

## LightOS integration

Ink adapts to available LightOS services, preferences and Android lifecycle. Host integration and distribution eligibility are separate contracts; see [LightOS](/light-sdk).

## Performance

Keep gestures and large media operations native. Batch updates, page large collections and avoid unnecessary polling.

### LP3 runtime benchmark

Use the [benchmark index](https://github.com/vandamd/ink/blob/main/benchmarks/README.md) for current app comparisons and the measurement limits.
