---
title: "How Ink works"
description: "How Ink builds and runs apps."
---

Ink runs React and TypeScript apps on Android. QuickJS-ng runs the JavaScript; Rust handles layout, input and rendering through Vulkan.

## Building an app

Ink checks TypeScript, transforms JSX, bundles JavaScript and prepares assets. It reads package dependencies from `package.json` and Android settings from `ink.toml`.

```toml
# ink.toml
name = "Weather"
package = "com.example.weather"
version = "1.0.0"
version_code = 1

[lightos]
enabled = true
```

The APK contains JavaScript source, icons, assets and the required native modules. Apps with background tasks also get a separate worker bundle.

Release builds load networking runtime code on first use. Set `INK_SPLIT_WEB=0` to disable this split for comparisons. General code splitting and downloading executable code after installation are not supported.

### Native release builds

Android releases use the speed-optimised `release` Cargo profile and the pinned toolchain's prebuilt standard library. Native libraries remain uncompressed in the APK so Android can map them directly.

Gradle uses its configured Android NDK and the app's minimum Android API level for native compilation. Missing native symbols fail at link time instead of producing an APK that fails to load.

### Native modules

Packages declare native requirements in `ink-native.json`. Ink combines these with capabilities configured by the app. A shared catalogue tells the compiler and Gradle which dependencies, permissions and Android sources to include.

Unused JavaScript can be removed from the bundle. Native SDKs may need to be included as a whole. Pure JavaScript packages need no Ink registration; third-party native modules do not yet have a general extension API.

For app-local Android code, an `android/build.gradle.kts` library can supply resources and a manifest. Ink compiles `android/kotlin/` with its Android host; that code provides `createAppNativeAdapters` for calls from `ink/native`. This integration uses internal Android interfaces and is not a portable package API.

## Running an app

React runs on a dedicated JavaScript thread. It manages app state and sends UI changes to Rust. Native API calls return promises so file and network operations do not block that thread.

QuickJS-ng manages its small-object allocation directly. Ink uses the standard runtime allocator, with a 64 MiB JavaScript memory limit and a 512 KiB stack limit.

The workspace pins `rquickjs-sys` to upstream commit `7a23f6f02f5396b41283b25a0d72f34d8982778e`, which bundles QuickJS-ng 0.17.0. The high-level bindings remain at rquickjs 0.14.0. Remove the Cargo patch when a published binding release includes this engine version.

`Text` and `Stack` are typed React host elements, so neither needs a component wrapper. JSX and `createElement` use the same path; these exports are host names, not callable component functions. Primitive Text content lives on its host node, while mixed and nested text use child nodes. The renderer resolves Text's `href` into a press handler and checks that links do not also supply `onPress`. Native Stack handles its optional press behaviour.

Rust measures text, positions components, handles gestures and draws the screen. It keeps the UI tree between updates and applies React changes in batches.

```text
React state changes
        ↓
Batched UI updates
        ↓
Rust layout and input handling
        ↓
Vulkan rendering
```

Scrolling continues natively while JavaScript is busy. New content and actions that need JavaScript must wait for it.

### Updating the screen

Changes to existing elements update the affected parts of the native tree. Structural and navigation changes rebuild the tree. Both recalculate layout.

Text changes send just the node ID and new text. Rust combines overlapping updates so rebuilding a parent also handles its changed children, then replaces the affected branches in one tree traversal. Text input synchronisation runs when inputs or the tree structure change, rather than on every text update.

JavaScript compares native-relevant props before constructing an outgoing prop object. Changes to children or callback identity alone do not require that object; callback references still update in JavaScript so events use the latest handler.

Adjacent text edits share one bridge operation. Native layout can measure a fixed-width, single-line text box without wrapping its contents first; drawing still handles wrapping, truncation and font fallback. Wrapped lines and prepared text share immutable strings to avoid copying them between caches and scenes.

Images with fixed bounds can finish loading without recalculating layout.

Lists render rows around the visible area and measure their heights automatically. Stable keys keep the scroll position when rows change. See [Lists](/lists).

The renderer reuses prepared text and image geometry. Text, icons and images share one textured pipeline. It combines adjacent draws of the same image without changing their order. A completed React update can replace a queued frame callback and render as soon as the GPU is ready. Active gestures, flings and scene animations stay paced by Android’s frame scheduler. Submission still goes through Android’s presentation pipeline; rendering immediately does not bypass the display’s refresh rate.

Input activation, haptic feedback and scene changes are separate signals. A tap that only dispatches a React callback does not redraw the old scene; the completed commit requests its frame.

An unchanged screen requests no rendering frames. Timers, network connections and media can still use CPU and power.

### Text and image caches

Text and icons share a texture cache that starts at 1 MiB. When it fills, Ink clears cached entries and rebuilds the current scene. If the scene needs more space, the cache grows to 4 MiB, then 16 MiB. Exceeding that limit reports a rendering error. Images use a separate cache.

## Leaving a screen

| Data or work | When it ends |
| --- | --- |
| Component state | Component unmounts. |
| Screen effects and resource subscriptions | Screen is hidden or removed. |
| Camera, microphone and foreground location | According to the module’s rules for hidden screens and app backgrounding. |
| Shared JavaScript state | App runtime stops. |
| Detached audio | Playback is stopped or Android terminates it. |
| Saved settings and queued jobs | Data is deleted or app storage is cleared. |

Putting the app in the background does not unsubscribe its screen from resources. Native modules handle backgrounding separately; check the module’s page for its behaviour.

Hooks release native resources when their effects end. For handles opened directly, call `close()` or unsubscribe when finished. Garbage collection does not close cameras, sockets or players. Reopen saved files by ID; old connection handles cannot be reused.

## Background work

[Background tasks](/background) run in separate JavaScript runtimes. They cannot access the foreground app’s variables or UI.

[Audio](/audio) and [Downloads](/downloads) run through native services and jobs without a continuous JavaScript loop. Android can stop them, so save the data needed to resume or retry.

## Dependencies and permissions

Packages share the app’s runtime, APIs and Android permissions. They are not isolated from each other. Review dependencies before adding them and keep the lockfile in version control.

Use [Secure store](/secure-store) for credentials protected by Android Keystore. For JavaScript hashing, use [`@noble/hashes`](https://github.com/paulmillr/noble-hashes).

[LightOS integration](/light-sdk) provides access to host services and preferences. Distribution approval is separate.

## Benchmarks

See the [benchmark results](https://github.com/vandamd/ink/blob/main/benchmarks/README.md) for app comparisons and measurement limits.
