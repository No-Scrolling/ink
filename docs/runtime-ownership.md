# Rust and JavaScript ownership

Reviewed 28 September 2026. Rust owns Ink's visual engine and native list windows. JavaScript supplies application data, actions and React semantics. Android services remain in Kotlin where they use platform APIs.

Android builds now select native list, message, playback and bound-view composition through the app's generated capabilities. Unused families can be removed without changing speed optimisation. Dynamic native-view descriptions conservatively retain broader support; custom React components keep their existing behaviour. See the [optional UI build results](../benchmarks/results/optional-ui-2026-09-28/README.md).

Canvas, text-input, camera, video and map primitives are also selected at build time. Core defaults retain all primitives; dynamic native views retain these paths because their descriptions can be constructed at runtime. The React commit decoder borrows QuickJS's JSON text until deserialisation finishes, then sends owned values across the thread boundary. See the [primitive and bridge cleanup results](../benchmarks/results/primitive-size-2026-09-28/README.md).

| Work | Owner | Behaviour |
| --- | --- | --- |
| Layout, text, drawing, hit testing | Rust, with Android fallback glyph support | No JavaScript drawing loop. |
| Dragging, momentum, scrollbar geometry | Rust; Android schedules frames | Compiled lists scroll without React window callbacks. |
| Images | Native request, decode, cache and rendering paths | Sources come from JS; image bytes bypass React. Native-only frames drain new image requests. |
| Text/Stack and standard Row list templates | Rust windows and identity; JS data dependencies | Supported inline declarations compile automatically. |
| Conversation, reorder and media picker lists | Rust windows and visual templates; JS data/actions | Ink's built-ins now use native row plans automatically. Message composition is native too. |
| Playback screen and progress labels | Rust composition and clock; native audio supplies anchors | `PlayingScreen` takes one playback binding and connects native clocks and transport commands. JS receives state changes; `getState()` reads a fresh snapshot. Source replacement and relative seeks preserve native timing. |
| Persistent view bindings | Rust targets; JS values | Explicit native views avoid repeated React host-tree reconciliation. |
| Custom React components | JavaScript execution and lifecycle; Rust resulting primitives | Hooks, effects, conditional trees and custom render functions retain React behaviour. |
| Controlled text input | Kotlin IME, Rust editor, JS value/actions | Event ordering and composition semantics remain intact. |
| Navigation, resources, filtering, fetching decisions | JavaScript | Application responsibilities. |
| HTTP, WebSockets and Blob | Native I/O and byte transport; JS Web API semantics | Body bytes use typed buffers, without base64/JSON byte encoding. Metadata still uses JSON. |
| Storage, SQLite, cryptography | Native operations; JS values/subscriptions | Large structured results can still incur JSON conversion. |
| Camera, video, maps, location, NFC, notifications, auth | Native services; JS configuration/actions | Thin adapters remain appropriate. |
| Video progress and time labels | Native playback clock and Rust visuals | Progress anchors bypass JS; React receives semantic state changes. |
| Recording duration, microphone levels and pitch | Native capture/analysis and readouts; optional status-only JS delivery | Existing hooks retain live measurements by default. Native readouts can update without React; custom measurement-driven UI still subscribes in JS. |
| Background work | Android scheduling, Rust runtime, JS task bodies | The binary bridge also works in worker runtimes. |
| Compiler and CLI | Rust orchestration and build-time JS transformation | Not part of the phone's UI update path. |

## Changes from the whole-runtime audit

**Built-in lists.** `ConversationScreen`, `ReorderList` and `MediaPicker` now declare native templates. Rust mounts and recycles visible rows; scrolling no longer asks React to build each window. Conversation messages use a native visual composite. Timestamp formatting, link detection, double-tap action recognition, fetching and sending remain JavaScript. The public component APIs are unchanged.

**Playback subscriptions.** Native-clock controllers receive meaningful state changes, including seeks, speed and queue changes. The native player sends clock anchors directly to Rust for elapsed-time labels and bars. `usePlayer` no longer sends periodic snapshots. JS `state.position` and `state.silenceSaved` are last-notified values; `getState()` reads fresh values when application logic needs them. See [native playback progress](audio.mdx#keep-visual-progress-native). Reverb's custom adapter uses the same native clocks and controller notifications instead of polling.

**Sparse list application.** Unchanged-order patches reuse the key index, update changed mounted rows and invalidate only their cached heights. Reorders retain the existing anchor-preserving path. This avoids full key-map and height-index reconstruction for a one-row edit. Shared version arrays can still require copying; JS still scans keys and visual dependencies on invalidated renders. This is not an O(1) end-to-end update claim.

**Binary transport.** Fetch chunks, managed-file reads, uploads and WebSocket binary batches travel beside the JSON envelope as byte arrays. Runtime queues own their data; cancellation and size limits remain. This removes base64 conversion, not every copy: QuickJS thread ownership and JNI still require byte copies. Native-file multipart parts retain their existing file-reference path. The unused JS base64 dependency was removed. No network throughput or APK-size improvement is claimed without a paired measurement.

## Why retain a React compatibility path?

A custom row may ultimately produce only Ink primitives while still running hooks, effects, context reads, arbitrary calculations or conditional component trees. Rust can own the resulting visuals without replacing those JavaScript semantics. Known declarations and Ink's built-ins use native plans; unsupported custom rows retain React rather than silently changing their behaviour.

Further compiler coverage should prove equivalence for pure wrappers. Dropping the compatibility path for all existing apps would require a narrower component contract or a substantially more capable compiler. Restricting drawing primitives alone does not prove that a component is stateless.

## Follow-up implementation and review

- Video now feeds the native playback clock, including pause, completion, seeking and buffering. Its progress bar and labels no longer take a periodic trip through React.
- `RecordingDuration`, `LevelReadout` and `PitchReadout` update their native text runs directly, without rebuilding layout. Capture hooks accept `updates: "status"` for these displays. Existing live measurement subscriptions remain the default for compatibility and custom application calculations.
- Compiled list projection now records changed rows and key order in the same pass. The renderer consumes that patch only when its base matches the committed items. Mismatched bases retain the comparison path. Consumed patches are discarded so they do not retain a chain of previous lists. Key/dependency scanning is still O(n); arbitrary application expressions remain JS.
- Map configuration uses normalised field comparison, removing the JS stringify/parse round trip and duplicate initial update. The native adapter still owns gestures and marker updates. Props are checked on each render so in-place marker edits remain observable.
- Complex bridge values retain the existing JSON semantics. The fresh instrumented React fixture measured about 0.051 ms in native transfer, including conversion and queue submission. This does not isolate arbitrary nested-value conversion. The earlier paired primitive converter gained roughly 2%, so a second general conversion path remains unjustified by the available measurements.

Rust now owns the standard continuously changing displays covered by this audit. JS still executes app functions, hooks, effects, callbacks and general React reconciliation. Keeping that compatibility path is deliberate. Further pure-row compiler coverage and native template allocation work need workload-specific evidence, not a blanket language migration.

The effects page still explicitly reads fresh player state once a second while visible. This is an app-specific statistic subscription, not a playback animation loop. A custom tuner can likewise retain JS measurements for its own note display policy. Kotlin remains appropriate for Android services and platform integration.

## Validation

See the [follow-up report](../benchmarks/results/ownership-followup-2026-09-28/README.md) for current paired list results, native video/recording evidence, bridge attribution and microphone test limitations.

See the [boundary implementation report](../benchmarks/results/native-boundaries-2026-09-28/README.md) for paired list measurements, emulator screenshots and playback commit counts. The expanded headless suite retains custom React state and callback checks alongside native built-ins. The emulator checks cover conversation visuals/actions, media selection/pagination, reorder actions, playback, networking and a background fetch. LP3 was not connected for this validation.

Earlier evidence: [list projection](../benchmarks/results/list-projection-2026-09-28/README.md), [automatic lists](../benchmarks/results/default-lists-2026-09-28/README.md), [native controls on LP3](../benchmarks/results/lp3-validation-2026-09-28/native-controls/README.md).
