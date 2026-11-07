---
title: "How Ink works"
description: "How Ink builds and runs apps."
---

Ink runs React-compatible TypeScript apps on Android with Preact. QuickJS-ng runs the JavaScript; Rust handles layout, input and rendering through Vulkan.

## Building an app

Ink checks TypeScript, transforms JSX, bundles JavaScript and prepares assets. It reads package dependencies from `package.json` and Android settings from `ink.toml`.

```toml
# ink.toml
name = "Weather"
package = "com.example.weather"
version = "1.0.0"
version_code = 1
```

Release APKs contain precompiled QuickJS bytecode, icons, assets and the required native modules. The CLI compiles the app, optional networking runtime and background worker with the same pinned QuickJS build as the Android runtime. Development builds retain JavaScript source for debugging and refresh.

Cargo also precompiles Ink's built-in runtime shims on the build host, using the same pinned engine. All builds load these shims as bytecode. Function source text is omitted; filenames and line tables remain for stack traces.

Rust reads installed app and networking bundles directly from APK assets. The app bundle loads on the JavaScript thread, avoiding source parsing and a Java string transfer through JNI at launch. Icons also load directly in Rust.

Release builds load networking runtime code on first use. Set `INK_SPLIT_WEB=0` to disable this split for comparisons. General code splitting and downloading executable code after installation are not supported.

### Native release builds

Android releases use the speed-optimised `release` Cargo profile and the pinned toolchain's prebuilt standard library. Native libraries remain uncompressed in the APK so Android can map them directly.

Gradle's `cargoBuildRelease` disables QuickJS's JavaScript parser and compiler with the target-specific C flag `QJS_DISABLE_PARSER`. Android release runtimes require precompiled Ink bytecode. Source bundles and development updates are rejected; JavaScript `eval` and `new Function` throw `TypeError: eval is not supported`. Libraries can catch this error and use a bytecode-compatible fallback. Development builds, host tools, the CLI's bytecode compiler, headless exports and host tests retain the parser.

Gradle uses its configured Android NDK and the app's minimum Android API level for native compilation. Missing native symbols fail at link time instead of producing an APK that fails to load.

### Native modules

Packages declare native requirements in `ink-native.json`. Ink combines these with capabilities configured by the app. A shared catalogue tells the compiler and Gradle which dependencies, permissions and Android sources to include.

Unused JavaScript can be removed from the bundle. Native SDKs may need to be included as a whole. Pure JavaScript packages need no Ink registration; third-party native modules do not yet have a general extension API.

For app-local Android code, an `android/build.gradle.kts` library can supply resources and a manifest. Ink compiles `android/kotlin/` with its Android host; that code provides `createAppNativeAdapters` for calls from `ink/native`. Adapters can send their own messages with `activity.emit`; listen for them with `onNativeMessage`, using a `type` that does not clash with Ink's own messages. This integration uses internal Android interfaces and is not a portable package API.

### App-local Rust

Put the app's Rust library in `android/rust/Cargo.toml` and provide an `rlib`:

```toml
[lib]
crate-type = ["rlib"]
```

Ink generates a wrapper crate in `.ink/android/rust/` that links the app and Ink
into `libink_android.so`. Export JNI functions with their existing names and load
the combined library from Kotlin:

```kotlin
System.loadLibrary("ink_android")
```

Ink uses the same wrapper build for apps without local Rust. Its host crate is an
`rlib`, so dependencies use Cargo's hashed cache files. The wrapper has a distinct
internal library name for each app and native configuration, then Gradle packages
it as `libink_android.so`. Switching between apps or capabilities cannot reuse a
different configuration's shared library.

When migrating a separate `cdylib`, remove its Cargo task and `jniLibs` source
directory from the app's Gradle script. Replace calls that load the old library
name. Ink rejects literal calls to the old name in `android/kotlin/` with migration
guidance. Code that calculates a library name must also use `ink_android`.

The app crate's default features remain enabled. Cargo unifies compatible shared
dependencies and their features; incompatible versions can remain separate Rust
dependencies inside the one library. Ink combines both workspace lockfiles,
selecting the newer compatible version of shared dependencies and retaining pins
for app-only dependencies, then asks Cargo to validate and resolve the graph.
Gradle builds that graph with `--locked`. Keep the app's
lockfile in version control and review dependency changes when updating either
the app or Ink.

The generated workspace combines both workspaces' Cargo patches, resolves patch
paths relative to their original workspace, and rejects conflicting patches.
The app workspace's profile settings override Ink's settings, including nested
package and build overrides. For example, the app's `opt-level = "s"` and
`lto = "thin"` apply to the combined release build. Without overrides, Ink uses
its usual release profile. One final link allows LTO across Ink and app code.
JNI symbol names must be unique across both crates. A combined library has one
`JNI_OnLoad`; apps that define it must initialise all their JNI registration there.

## Running an app

Preact runs on a dedicated JavaScript thread. It manages app state and sends UI changes to Rust. Native API calls return promises so file and network operations do not block that thread.

Rust schedules store and read-only SQLite requests on worker threads. A Kotlin backend calls Android's built-in SQLite synchronously on those threads, and Rust sends results directly to the JavaScript queue. Android APKs do not bundle a SQLite engine. Encryption runs as a Rust native function. These modules retain the native request timeouts, cancellation and result-size limits. Android services such as Keystore, networking, camera and location use Kotlin adapters.

QuickJS-ng manages its small-object allocation directly. Ink uses the standard runtime allocator, with a 64 MiB JavaScript memory limit and a 512 KiB stack limit.

The workspace pins `rquickjs-sys` to upstream commit `7a23f6f02f5396b41283b25a0d72f34d8982778e`, which bundles QuickJS-ng 0.17.0. The high-level bindings remain at rquickjs 0.14.0. Remove the Cargo patch when a published binding release includes this engine version.

`Text` and `Stack` are typed host elements, so neither needs a component wrapper. JSX and `createElement` use the same path; these exports are host names, not callable component functions. Primitive Text content lives on its host node, while mixed and nested text use child nodes. The renderer resolves Text's `href` into a press handler and checks that links do not also supply `onPress`. Native Stack handles its optional press behaviour.

Rust measures text, positions components, handles gestures and draws the screen. It keeps the UI tree between updates and applies Preact changes in batches.

```text
Component state changes
        ↓
Batched UI updates
        ↓
Rust layout and input handling
        ↓
Vulkan rendering
```

Scrolling continues natively while JavaScript is busy. New content and actions that need JavaScript must wait for it.

### JavaScript compatibility

Apps keep importing from `react` and using `@types/react`. The compiler aliases React imports and JSX runtimes to Preact, and React DOM imports to Preact’s compatibility modules. Ink’s own components use Preact core; its memoised rows use a class with `shouldComponentUpdate`.

The React Compiler still targets React 19. Ink supplies the compiler’s per-component memo cache through Preact hooks. React’s reconciler and scheduler are absent from app bundles. The native operation protocol is unchanged.

Development uses Prefresh with Oxc’s refresh registrations and hook signatures. Component body edits retain state; changing the hook signature resets it. The existing development module cache and full-reload compatibility checks remain in place. Ink defers the development root update until Prefresh has replaced mounted component types.

Preact updates run to completion on the JavaScript thread. React’s concurrent scheduling is not available: transitions run synchronously and deferred values update immediately. Ink does not provide a browser DOM, server rendering or React Server Components.

### Updating the screen

Changes to existing elements update the affected parts of the native tree. Structural and navigation changes rebuild the tree. Layout reuses measurements and scene ranges for unchanged static subtrees when their available rectangles and drawing context match. Changed content, bounds, clipping, scale or appearance takes the normal layout path.

Lists, images, text inputs, platform surfaces and animated readouts retain their layout side effects. Static children inside these compositions can still reuse their output. Ink keeps the previous primitive buffers and remaps cached text indices and copies hit regions as it builds the next scene.

Text changes send just the node ID and new text. Rust combines overlapping updates so rebuilding a parent also handles its changed children, then replaces the affected branches in one tree traversal. Text input synchronisation runs when inputs or the tree structure change, rather than on every text update.

JavaScript compares native-relevant props before constructing an outgoing prop object. Changes to children or callback identity alone do not require that object; callback references still update in JavaScript so events use the latest handler.

Adjacent text edits share one bridge operation. Native layout can measure a fixed-width, single-line text box without wrapping its contents first; drawing still handles wrapping, truncation and font fallback. Wrapped lines and prepared text share immutable strings to avoid copying them between caches and scenes.

Images with fixed bounds can finish loading without recalculating layout.

Lists render rows around the visible area and measure their heights automatically. Stable keys keep the scroll position when rows change. See [Lists](/lists).

The renderer reuses prepared text and image geometry. Text, icons and images share one textured pipeline. It combines adjacent draws of the same image without changing their order. A completed Preact update can replace a queued frame callback and render as soon as the GPU is ready. Active gestures, flings and scene animations stay paced by Android’s frame scheduler. Submission still goes through Android’s presentation pipeline; rendering immediately does not bypass the display’s refresh rate.

Input activation, haptic feedback and scene changes are separate signals. A tap that only dispatches a JavaScript callback does not redraw the old scene; the completed commit requests its frame.

An unchanged screen requests no rendering frames. Timers, network connections and media can still use CPU and power.

### Text and image caches

Text and icons share a texture cache that starts at 1 MiB. When it fills, Ink clears cached entries and rebuilds the current scene. If the scene needs more space, the cache grows to 4 MiB, then 16 MiB. Exceeding that limit reports a rendering error. Images use a separate cache.

## Leaving a screen

| Data or work | When it ends |
| --- | --- |
| Component state | Component unmounts. |
| Ink hook effects and resource subscriptions | Screen is hidden or removed. |
| App effects and subscriptions from `react` | Component unmounts, or the app cleans them up. Hiding a screen keeps them active. |
| Camera, microphone and foreground location | According to the module’s rules for hidden screens and app backgrounding. |
| Shared JavaScript state | App runtime stops. |
| Detached audio | Playback is stopped or Android terminates it. |
| Saved settings and queued jobs | Data is deleted or app storage is cleared. |

Ink keeps hidden screens mounted and sends native visibility changes without adding layout nodes. Ink’s resource, controller and native module hooks disconnect their effects and subscriptions while hidden, then reconnect when shown. Component state is retained. Unlike React’s `Activity`, this does not pause an app’s own `useEffect`, `useLayoutEffect` or `useSyncExternalStore` calls imported from `react`. Clean up work that should stop when leaving a screen, or keep that work in an Ink resource or controller hook.

Putting the app in the background does not unsubscribe its screen from resources. Native modules handle backgrounding separately; check the module’s page for its behaviour.

When the Android surface disappears, Ink releases its window and swapchain but keeps the Vulkan device, pipelines, glyph atlas and image textures while the activity remains alive. Returning to the app recreates the surface resources. Destroying the activity releases the renderer. Pipeline cache data is saved in Android’s code cache for later launches.

Hooks release native resources when their effects end. For handles opened directly, call `close()` or unsubscribe when finished. Garbage collection does not close cameras, sockets or players. Reopen saved files by ID; old connection handles cannot be reused.

## Background work

[Background tasks](/background) run in separate JavaScript runtimes. They cannot access the foreground app’s variables or UI.

[Audio](/audio) and [Downloads](/downloads) run through native services and jobs without a continuous JavaScript loop. Android can stop them, so save the data needed to resume or retry.

## Dependencies and permissions

Packages share the app’s runtime, APIs and Android permissions. They are not isolated from each other. Review dependencies before adding them and keep the lockfile in version control.

Use [Secure store](/secure-store) for credentials protected by Android Keystore. For JavaScript hashing, use [`@noble/hashes`](https://github.com/paulmillr/noble-hashes).

[LightOS integration](/light-sdk) provides access to host services and preferences. Distribution approval is separate.
