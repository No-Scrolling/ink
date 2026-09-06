# Completing Ink's developer experience

Status: framework implementation delivered, with focused verification completed on 6 September 2026. Physical LP3 counter/scroll benchmarks are now complete. Real-app workflows remain explicitly deferred for a joint session with the user. The detailed sections below preserve the intended design and its acceptance criteria; they are not a claim that every device scenario has been exercised.

## Current implementation status

| Step | Status | Evidence and remaining work |
| --- | --- | --- |
| 1. Predictable builds and assets | Implemented | Resolved-graph `ink-native.json` declarations, a shared capability catalogue, imported media/icon collections and a manifest consumed by watching and `ink info`. Counter and benchmark release checks pass. See [build contracts](build-contracts.md). |
| 2. Native requests and lifetimes | Implemented; device stress deferred | Shared foreground/worker bookkeeping, stable errors, process-wide identifiers, controller ordering, fair timers and explicit queue failures. Rust and full-capability Kotlin compilation pass. See [native contracts](runtime-contracts.md). |
| 3. Development reload and refresh | Implemented; focused emulator checks passed | ADB bundle generations, compatible React Refresh, graph watching, mapped errors and immutable worker code. Emulator checks cover state-preserving edits, hook resets, linked modules and exact-location error recovery without APK installation. See [development](development.md). |
| 4. Shared screen behaviour | Implemented; bounded verification | Prior template emulator work covers appearance, navigation, keyboard transitions, wrapping fields and common screen patterns. The [public-prop audit](public-props-audit.md) traces public interfaces to implementation and removes stale claims. Delayed input and wider composition/device stress are not signed off. |
| 5. Variable-height lists | Implemented; focused fixture checks passed | Estimated heights, key/content/width measurement caches, anchored scrolling, bounded windows and optional end-following. Expansion, scrolling and prepend anchoring were checked in the emulator; fixed-height updates still work. See [lists](lists.md). Physical performance remains unmeasured. |
| 6. Compatibility and standalone installation | Implemented for an explicit local SDK | Shared runtime typings/configuration, `ink create`, explicit SDK discovery and per-app Android output isolation. An app outside the repository installed dependencies, checked, launched in the emulator and produced signature-verified debug/release APKs. Public registry naming/publication remain release decisions. See [standalone setup](standalone.md) and [runtime compatibility](runtime-compatibility.md). |
| 7. Real workflows and documentation | Documentation updated; workflows deferred by request | Compiler/runtime/development/list/distribution guides and API claims updated. Physical LP3 release counter/scroll benchmarks are complete. Passes, Spotify and Beeper-style workflows and wider dependency exercises will be done with the user later. |

See [the verification record](verification-2026-09-06.md) for commands, scenarios and limits. No automated tests were added. Compilation and focused emulator scenarios establish only the behaviours actually exercised. The [physical LP3 benchmark report](../benchmarks/results/ink-react-lp3-2026-09-06.md) now records release counter/scroll measurements, with [fresh Expo and Light SDK comparisons](../benchmarks/results/expo-light-sdk-lp3-2026-09-06.md). These fixtures do not establish variable-height list performance, input-to-photon latency, battery life or production readiness.

## Outcome

An author can create an app outside this repository, write ordinary TypeScript and React, compose Ink screens, use supported device packages and iterate on a physical Light Phone III without learning the renderer's protocol.

Keep React, QuickJS-ng on its dedicated thread, native Rust layout and interaction, Vulkan rendering and separate background runtimes. Accessibility, React Native compatibility and automatic LightOS colour matching are outside scope. The phone investigation established that the inspected SDK does not expose its invert-colours preference.

Existing TypeScript interfaces are the starting point. New interfaces described here are proposals. Asset and package changes require migration of the examples and documentation.

## 1. Predictable builds and assets

**Files:** `crates/ink-compiler/src/{bundle-javascript.js,javascript.rs,capability.rs}`, package exports and `platform/android/app/build.gradle.kts`.

**Problem:** native inclusion currently depends on recognising JSX names, hook imports and option expressions. Icons depend on discovering string literals. Equivalent React code can produce different native builds.

**Implementation:**

- Give each supported package a small versioned native-requirements declaration. Discover requirements through the actual resolved module graph, including aliases, re-exports, linked packages and workers. Start conservatively at package/export granularity. Including some extra native code is preferable to silently excluding a feature; expose that cost in `ink info`. Split exports only when their cost justifies the extra interface.
- Keep explicit `ink.toml` capabilities additive. Establish one capability catalogue for names, dependencies, permissions and Android source groups, consumed by the compiler and Gradle. Keep build actions in Gradle rather than inventing a build language.
- Make image/audio imports the normal asset mechanism. Stop rewriting strings inside JSX. Runtime HTTPS and managed-file URLs remain values; local files must be imported or explicitly included.
- Make icons explicit imported assets, with the same reference accepted by Icon, Button and Tab. References can travel through ordinary props and maps. Include framework-owned icons with the framework. For names supplied by external data, allow an explicitly included icon collection with a documented missing-name result.
- Retain build-time icon masks initially. Declare asset raster resolution independently of JSX call sites and scale for display, with documented size limits. A new runtime font/vector subsystem is not required.
- Emit a bundle manifest containing resolved inputs, assets, capabilities, worker presence and framework/protocol version. Reuse it for watching, rebuild decisions and `ink info`.

**Delete:** feature inference from JSX/hook names, option-object inspection, icon string hunting and duplicated capability tables after migration. Do not retain two competing discovery paths.

**Complete when:** wrappers, aliases, re-exports, `createElement` and extracted options cannot silently omit native requirements. Assets selected from imported collections work regardless of component structure. Diagnostics explain missing declarations.

## 2. Consistent native requests and resource lifetimes

**Files:** `packages/ink/src/{native.ts,controller.ts}`, device hooks, `platform/android/native/src/{javascript.rs,worker.rs}`, and Kotlin `NativeAdapter.kt`, `MainActivity.kt`, `InkWorker.kt`.

**Implementation:**

- Document calls, results, cancellation, controller events, commits and shutdown as one message contract. Use explicit stable error codes: Kotlin's enum-name conversion currently differs from Rust's hyphenated names. Retain JSON.
- Share the request bookkeeping duplicated by the foreground and worker hosts: registration, exactly-once settlement, timeout, cancellation and disposal of late results. Each host supplies its actual adapters; workers do not acquire UI facilities.
- Scope callbacks, request IDs and controller handles to a runtime session. Restarting JavaScript must not let an old completion reach a new owner with a reused numeric ID.
- Define activation/command/disposal ordering, including failed activation and disposal while activation is pending. Disposal is idempotent. Session shutdown releases attached resources and observers.
- State each package's ownership: mounted component, visible screen, suspended app, detached native service or durable job. Do not alter ordinary React effects or silently cancel arbitrary app promises. Ink-owned hooks manage their native resources; explicit handles expose close/unsubscribe operations.
- Define queue-full behaviour. Calls may reject with a useful busy error; commits must not disappear silently. Start with explicit failure/recovery rather than a new transport.
- Give timers, promise jobs and incoming native messages fair turns. The current loop fires timers only when waiting for a command times out; sustained messages can delay already-due timers. Service due timers independently of whether a message arrived, while keeping shutdown interruptible.

**Complete when:** rapid navigation, cancellation, pause/resume and runtime restart cannot leak attached resources or deliver stale results. Foreground and worker errors agree. Detached audio survives UI disposal as documented.

## 3. Fast development without APK reinstalls

**Files:** `crates/ink-cli/src/{main.rs,android.rs,watch.rs}`, compiler profiles, `ink-runtime`, Android startup and lifecycle.

Implement in two increments:

1. **Bundle reload:** install the development host once, transfer new JS/assets over an ADB-backed connection into app-private storage, activate a complete bundle generation and restart only the UI runtime. Initially this resets React state. Failed builds leave the current app running; activation failures show a recoverable developer error. Native capability, manifest, framework or native-source changes trigger an APK rebuild. Pin worker bundles when jobs start so active jobs finish on their original generation.
2. **State-preserving refresh:** add React refresh instrumentation and module-update support once reload is reliable. Preserve state for compatible component edits and fall back to reload otherwise. The current IIFE bundle cannot provide module updates just by enabling a flag.

Use development React, readable output and source maps in development; keep release minification. Map errors to original TS/TSX locations and provide a reload action after runtime failure. Omit the development transport from release builds.

Watch resolved compiler inputs, including linked packages outside the app root. Watch dependency/config changes that invalidate the graph. Do not continuously scan `.git`, build outputs or all dependencies.

**Complete when:** screen and linked-package edits appear without Gradle/install, errors have useful locations, recovery does not require restarting the CLI, and native changes trigger only the necessary rebuild. No callbacks survive into the wrong runtime generation.

## 4. Complete the shared screen behaviour

**Files:** `packages/ink/src/{index.ts,navigation.ts}`, `crates/ink-core/src/{lib.rs,react.rs}`, renderer clear colours, Android keyboard integration and template settings.

**Implementation:**

- Add a small app appearance interface backed by native light/dark palettes. Apply it to text, controls, backgrounds, navigation, scrollbars and the keyboard where supported. Preserve image/camera pixels and intentional barcode contrast. Changing appearance must repaint without remounting screens.
- Wire the template's existing colour setting to appearance and persist it with `@ink/store`. Do not expose a misleading system-following mode: the current SDK supplies no global LightOS colour preference.
- Make header back, edge gestures and hardware back share navigation and keyboard-dismiss rules. Preserve route/tab state and scroll positions. Implement invalid external-route behaviour consistently with the documentation.
- Verify controlled input under delayed JS updates: composition, cursor position, deletion, submit, focus and keyboard viewport changes. Retain native editing and event counters.
- Complete settings-choice, loading/error/empty and confirmation patterns by composing existing components first. Add native primitives only for interactions the current layout cannot express.
- Audit every public prop and documentation claim against implemented behaviour. Remove unsupported claims rather than accept props that do nothing. Accessibility remains removed.

**Complete when:** appearance affects the whole app, back behaviour is consistent, keyboard use preserves visibility and focus, and reference screens require no app-specific layout machinery for common states.

## 5. Variable-height lists and stable scrolling

**Files:** `packages/ink/src/list.ts`, native `ReactList` layout and `ReactTree::viewport_events`.

Keep fixed-height lists as the simple fast path. Add an estimated-height mode using the existing items, keys and renderItem model; authors must not calculate message heights.

**Implementation:**

- Store native measurements by list identity and stable item key. Invalidate on content/layout revisions and width or typography changes.
- Maintain estimated extents for unmounted rows and an offset index for viewport lookup. Send ordered keys and changes, not complete item objects, to Rust. Bound metadata transfers to the runtime's message limits.
- Preserve a visible anchor consisting of an item key and pixel offset when earlier rows are inserted, removed or remeasured. Define what happens if the anchor is deleted. Provide explicit chat behaviour that follows new messages only when already near the end.
- Publish windows only when they change, with overscan and a list revision to reject stale events. Native scrolling continues during JS work, but uncached content may wait for JS; document that limitation.
- Preserve identity for rows that stay mounted. Durable state for unmounted rows remains in app state/storage.

Structural commits currently rebuild the rendered tree. Keep the fallback initially and measure append/prepend performance. Add structural subtree updates if device results require them; a complete incremental-layout rewrite is not a prerequisite for correctness.

**Complete when:** mixed-height messages, prepended history and completed image loads do not move the reader unexpectedly; mounted rows stay bounded; fixed-height behaviour is preserved.

## 6. Honest package compatibility and standalone installation

**Files:** runtime globals and typings, `packages/ink/src/{web.ts,http.ts,fetch.ts,blob.ts,websocket.ts}`, package metadata, app tsconfig and CLI framework discovery.

**Implementation:**

- Inventory supported globals and limitations. Ship runtime typings and a shared app tsconfig based on ECMAScript plus implemented facilities, rather than implicitly advertising the entire DOM. Check third-party declarations for accidental reintroduction of unavailable globals.
- Demonstrate selected dependency patterns used by these apps: hooks/context, pure-JS state, data decoding and networking. Installing successfully does not establish runtime compatibility. Keep Node, DOM and React Native native-module requirements explicit.
- Keep bulk media native where URIs/streams suffice. Document that current HTTP uploads consume input before opening the request; true live request streaming is separate work if an app needs it.
- Replace the CLI's compiled-in repository location with a versioned SDK installation or explicit development checkout. Isolate per-app build/output paths so applications do not overwrite shared Android artefacts.
- Add minimal app creation, a compatible framework/React/reconciler/package version set and migration documentation. The first distribution can still require Bun, Rust, JDK, Android SDK and NDK, diagnosed by `doctor`. Consider prebuilt binaries only after measuring setup/build costs.
- Resolve public package names and registry availability before publication. Publishing and release signing are separate release actions.

**Complete when:** an app created outside this checkout can install dependencies, check, launch on LP3 and produce a signed release using supplied signing configuration. Supported dependency examples execute on-device.

## 7. Verify real workflows and align the documentation

Use bounded representative flows instead of porting all existing apps at once:

| Flow | Establishes |
| --- | --- |
| Template settings/search | Composition, navigation, input, appearance and reload |
| Passes-style scan/saved barcode | Camera ownership, storage and native images |
| Spotify-style queue/playback | Async interfaces, detached work and recovery |
| Beeper-style conversation with local fixtures | Variable heights, live updates, scrolling and images |

The Spotify Android SDK wrapper is distinct from generic audio. Port it only if that specific integration is needed; arbitrary Expo module compatibility remains outside scope. Use fixture data for list work to avoid coupling renderer verification to backend/authentication work.

Record physical LP3 release APK size, startup-to-usable-screen, idle memory, input responsiveness, list frame behaviour and sustained resource use. Measure development edit-to-visible-update separately. Establish baselines before setting numeric budgets. The earlier synchronous counter does not measure the full React architecture; state-update time is not input-to-photon latency, and idle frame counts do not establish battery life.

Use compilation, TypeScript/build checks and documented manual device scenarios. Do not add automated tests under the current user instruction unless requested. Compilation alone cannot establish behaviour.

Update README, architecture/core/package docs and the reference app alongside each change. Audit existing claims about appearance, dynamic icons, image sizing and invalid routes. `CONTEXT.md` has been corrected during planning; implementation status belongs in the documentation, not the glossary.

## Delivery order and finish line

1. **Reliable contracts:** steps 1 and 2; migrate examples and delete replaced machinery.
2. **Daily development:** bundle reload and mapped errors, followed by shared screen behaviour.
3. **Real content:** variable-height lists and dependency compatibility, verified through representative flows.
4. **External developers:** finish standalone installation and creation, and state-preserving refresh.

The first usable milestone is reliable contracts, bundle reload and the complete template. The broader promise also needs measured lists, demonstrated dependency support, state-preserving refresh and standalone installation.

Full React DevTools integration, more native packages, binary transports, finer native tree shaking and incremental layout should be justified by actual use or measurements. No engine replacement, general CSS layer, React Native emulation, automatic LightOS appearance detection or automation platform is required.
