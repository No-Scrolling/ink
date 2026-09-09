# Light-template rapid tab profile — 9 September 2026

Rapid switching has two distinct problems: measurable response latency on the physical Light Phone III, and input handling that can discard taps. Overlapping fingers reproduce a particularly severe failure independently of rendering speed.

## Build and method

- Profiled the working tree's `examples/light-template`, Home ↔ Settings, in isolated release builds made with `scripts/agent-tools`.
- Timing experiment: `experiment-20260909-002041-56cf8be6`, `INK_PRESENTATION_TIMING=1`, source hash `65b5947def891e038130a14608eeedbdc7a96d8e33e26ee253395e5defd16e09`.
- CPU experiment: `experiment-20260909-002556-db89b05b`, `INK_BENCHMARK=1` without presentation instrumentation, source hash `67f18159b469389201db677091338d18ae95626229cbb8dcec68bf7768096413`.
- Only the package identifier was temporarily changed (`com.vandam.ink.tabprofile` / `com.vandam.ink.tabcpu`) to preserve existing installations. The example manifest was restored. No implementation changes or tests were made.
- Physical device: `LP3LHMA531900140`, Light Phone III, Android API 34, 1080×1240; thermal status 0 before and after profiling.
- Also profiled `emulator -avd Light_Phone_III -writable-system`, API 34. Emulator performance is not representative of the physical phone.
- A single `app_process` Java injector sent Android touchscreen MotionEvents at x=1000 (Settings) / x=80 (Home), y=1150. Clean taps held down for 20 ms at nominal 250, 100 and 50 ms start intervals. Saved injector timestamps record actual cadence. No per-tap ADB process overhead.
- Runs start on Home, first tap Settings, alternate and end on Home. Screenshots establish the hit locations and expected pages.

## Physical phone timings

| Tap start interval | Taps | Submitted frames | Release → submission median / p95 | Release → driver display median / p95 |
|---|---:|---:|---:|---:|
| 250 ms | 20 | 21¹ | 32.1 / 37.5 ms | 55.7 / 67.0 ms |
| 100 ms | 60 | 60 | 32.1 / 36.9 ms | 56.8 / 64.8 ms |
| 50 ms | 100 | 83 | Not safely attributable² | Not safely attributable² |

¹ First Settings visit also resolves its data and submits another frame. Timing uses the first frame of each switch.

² At 50 ms, only 92 native React dispatches occur for 100 final input releases; commits can cross interaction boundaries. A simple nearest-event attribution is ambiguous, so do not use that run's provisional latency numbers as per-tap measurements. Frame and dispatch totals are directly counted. Coalescing and reselecting an already active tab can also explain some missing frames; 17 missing frames does not mean 17 individually proven discarded input events.

At 100 ms, median stages were:

| Stage | Median |
|---|---:|
| Native release → React dispatch | 0.95 ms |
| React dispatch → JS outgoing message | 21.85 ms |
| JS ready → native commit application | 2.66 ms |
| Native commit application | 0.77 ms |
| Commit complete → render submission log | 5.96 ms |

Stage medians do not sum exactly to the median total. `ReactReady` can represent a native request rather than a commit; the settled 100 ms run has one matching commit/frame per tap. First-visit data loading is not a clean JS-only measurement.

Display timings match `Presented id` to later `Presentation id actual_ns` feedback, including feedback collected in subsequent runs. All 20 and 60 first switch frames have feedback. These measure Android software/driver presentation after native handling of finger release, not touch sensing or physical panel response. Submission log timestamps have millisecond precision; they are not display timestamps.

## CPU trace

A separate ordinary benchmark build ran 60 taps at 100 ms with Android atrace (`gfx input view sched freq`). Median native frame wall time was 2.91 ms; relayout 0.24 ms, preparation 0.19 ms and combined submission/presentation 1.19 ms. GPU command spans in the logs are sub-millisecond. Full native tree rebuilds occur on tab changes, but layout is not the largest measured cost here.

The runtime worker (TID 25558, named `Thread-3` after JNI attachment) accumulated substantial CPU work in the tap windows: median 8.22 ms, p95 40.65 ms across 59 complete inter-tap windows. Its wake-to-running delay was median 0.024 ms, p95 0.564 ms. These windows include background/passive work and are not isolated React render durations. They support investigating JS/runtime work rather than attributing the whole delay to waiting for the worker to be scheduled. Function-level CPU sampling was unavailable: Android denied the simpleperf CPU event. This profile does not identify a particular React function or prove that QuickJS alone is responsible.

## Reproduced overlapping-finger failure

Twenty pairs, 40 finger contacts total:

1. Settings finger down, wait 40 ms.
2. Home finger down while Settings remains held, wait 20 ms.
3. Settings finger up, wait 40 ms.
4. Home finger up, wait 200 ms.

Both emulator and phone produced **20 final native releases, 20 React dispatches, zero commits, zero submitted frames**. Home remained selected throughout the run; final screenshots confirm Home.

`platform/android/app/src/main/kotlin/com/vandam/ink/MainActivity.kt:1175` returns early whenever `event.pointerCount > 1`, even when no image pinch is active. This discards the second finger's `ACTION_POINTER_DOWN` and the first finger's `ACTION_POINTER_UP`. Only the final Home `ACTION_UP` reaches the engine. `crates/ink-core/src/lib.rs:1744` then hit-tests that final release against the existing content pointer. Consequently it reselects Home and never activates Settings. This is an input loss problem; rendering cannot make those discarded taps responsive.

## Additional rapid-tap cancellation path

`crates/ink-core/src/react.rs:424` resets `engine.pointer = None` whenever the active screen identity changes. If a previous tab switch commits after the next finger goes down but before it comes up, this erases the next gesture. `pointer_up` immediately returns when there is no pointer. This is a concrete code path consistent with the eight undispatched releases in the phone's 50 ms workload. Without pointer-down/commit correlation instrumentation, the trace does not prove that it accounts for every missing dispatch.

## What this says about Expo / Beeper

Beeper uses Expo Router tabs, disables tab animation, and renders individual React Native `Pressable` tab buttons (`app/(tabs)/_layout.tsx`, `components/Navbar.tsx`, `components/HapticPressable.tsx`). Ink uses a single SurfaceView and its own pointer handling. The blanket multi-pointer early return belongs to Ink's implementation.

Ink's tabs already retain pages using React `Activity`; saying it mounts everything from scratch on each tap would be inaccurate. Visibility changes do reconnect effects, and selection goes through the JS navigation/reconciliation/commit path before the active native tab changes.

Beeper itself was not benchmarked, so this is not a measured Expo-versus-Ink latency comparison. The results establish Ink's own delay and reproduce input loss that plausibly explains the reported difference, especially during two-thumb switching.

## Recommended follow-up

1. Track tab presses by pointer ID and process intermediate pointer releases while preserving genuine pinch handling. Do not globally suppress all multi-finger input.
2. Preserve an in-progress tab-bar gesture across a content-screen commit; distinguish persistent navigation controls from gestures belonging to the outgoing page.
3. Profile the JS tab switch and Activity/effect work more deeply; measure any optimisation on the phone. Native layout optimisation alone will not remove the observed ~22 ms dispatch-to-ready stage.

## Local evidence

Evidence is retained in `.agent-tools/tab-profile/` (ignored local files):

- `TabTaps.java`, `OverlapTaps.java`: reproducible input workloads.
- `log-{250,100,50,overlap}.txt`, `taps-*.txt`, `summary.json`: emulator measurements.
- `phone/log-{250,100,50,overlap,flush}.txt`, `phone/taps-*.txt`: phone timing measurements and delayed driver feedback.
- `phone/presentation-summary.json`, `phone/summary.json`: timing summaries; see the 50 ms attribution caveat above.
- `phone/trace.txt`, `phone/log-cpu.txt`, `phone/trace-summary.json`, `phone/scheduling-summary.json`: ordinary benchmark CPU/scheduler evidence.
- `home.png`, `settings.png`, `after-overlap.png`, `phone/home.png`, `phone/after-overlap.png`: screenshots.
- Experiment directories retain APKs, source snapshots, hashes and build logs.

## Input handling fix and verification

Implemented after the profile:

- Android tab-bar gestures now track each pointer ID separately. Native tab presses retain their original action across content commits and validate it against the current tab bar on release. Removed tabs, cancelled gestures and drags do not activate a replacement target. Content scrolling and image pinch handling retain their existing path.
- The keyboard had a separate single-pressed-key implementation. It now tracks keys per pointer ID, processes intermediate pointer releases and keeps backspace repeat tied to the fingers actually holding it. Cancellation, detachment and keyboard-mode changes clear held keys.
- Final Android release build: `experiment-20260909-003537-c5c50e15` (`INK_PRESENTATION_TIMING=1`, temporary package `com.vandam.ink.inputfinal`). The example manifest was restored afterwards.
- Emulator replay: 40 overlapping tab taps plus 100 taps at 50 ms intervals produced 140 native releases, 140 React dispatches and 140 submitted frames. The first Settings data load adds one extra commit.
- Keyboard replay: five overlapping p/q pairs entered `pqpqpqpqpq`; a subsequent backspace left `pqpqpqpqp`. Evidence is in `.agent-tools/input-fix/`, including `final-log.txt`, injector timestamps and keyboard screenshots. Some emulator screenshots captured incomplete/black composition; `keyboard-after.png` and `final-backspace.png` contain the visible text.
- `cargo check -p ink-core`, the Android build and `git diff --check` passed. No tests were added. The physical device was left untouched for the user's `ink dev` verification.

This fixes demonstrated input loss. It does not optimise the JS/runtime latency measured above or establish that every aspect of awkward typing has the same cause.

## Shared input implementation

The tab-specific implementation above has now been replaced by shared Ink core pointer handling. Every Android pointer event carries its finger ID through the existing native input entry point. The core tracks each pointer's gesture and original hit target; there is no separate tab JNI method or tab gesture branch.

Hit targets use the React host identity and event role, independent of changing event arguments. Native input, clear, seek and back controls also have stable target roles. Releases are validated against the current target. A surviving control keeps its pending press across commits, whereas a hidden, removed or disabled target cancels it. Validation occurs after final layout and virtualised-list anchor correction. Pending presses are distinguished from active drags when the screen changes.

Scroll, text drag, image pan, scrollbar and edge-back gestures arbitrate ownership in the core. Long presses claim their gesture, and pinching cancels tap pointers. Android retains frame-coalesced movement and per-finger long-press timers. The native keyboard retains the per-finger key handling already implemented.

Shared builds:

- `experiment-20260909-004655-0cf3dc4a`: initial shared implementation; ordinary Add Items, Toggle and removal replays.
- `experiment-20260909-005114-4fcab4e4`: final shared implementation, with pending edge/input press preservation and validation after layout correction; temporary package `com.vandam.ink.sharedfinal`. The manifest was restored afterwards.

Manual emulator verification, retained under `.agent-tools/shared-input/`:

- Final build: 40 overlapping tab taps plus 100 rapid taps yielded 140 native releases, 140 React dispatches and 140 submitted frames (`final-tab-log.txt`).
- Two overlapping Add Items presses produced two dispatches, two commits and Item 1 / Item 2 (`button-log.txt`, `two-items.png`).
- Two overlapping Show Items toggle presses both dispatched and committed despite the first changing the next event's boolean argument (`toggle-log.txt`, which also contains the preceding Add Items pair).
- Hiding Add Items while another finger held it produced two input releases but only one dispatch/commit (`removed-log.txt`). The removed button did not fire.
- Final build: scrolling navigated the Examples list; image pinching changed the zoom and a subsequent drag panned it (`image-*.png`, `image-gesture-log.txt`).
- Final build: holding a conversation message opened Message actions, with one dispatch and no additional release action (`long-press.png`, `long-press-log.txt`).
- `cargo check -p ink-core`, the Android release build and `git diff --check` passed. The final experiment's four changed Rust/Android input source files were compared byte-for-byte with the working tree.

No tests were added. The physical phone was not changed during implementation; device verification with `ink dev` remains with the user. These checks establish the shared input behaviour, not a new physical-device latency result.
