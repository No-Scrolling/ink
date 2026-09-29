# Headless update benchmark

Run the existing `ink-updates` app through the real QuickJS runtime, React renderer, native event queues, commit bridge and Rust layout engine. No emulator, Android SDK or GPU is needed. Requires the repository's Rust toolchain, Bun and installed workspace dependencies; Hyperfine is optional.

```sh
scripts/agent-tools headless --hyperfine --background
scripts/agent-tools wait HEADLESS_ID --timeout 30
scripts/agent-tools result HEADLESS_ID
```

The tool builds a release harness and compiles the normal production app bundle before measuring anything. It copies the binary and two app assets into the result directory so subsequent runs cannot accidentally use a different build. No repository snapshot or Android build workspace is created. Results include source/artifact hashes, stage timings, a report and Hyperfine's JSON output.

Each process mounts the app at a 1080×1240 viewport, selects 500 cells and enables resize, then discards 20 warm-up updates and measures 200 updates. Change these with `--warmup` and `--iterations`. `--runs` sets Hyperfine's repetitions (default five, plus two whole-process warm-ups).

## What the numbers mean

Use `--app benchmarks/apps/ink-views` to measure the native-binding adapter. React mounts this page, but measured updates send a tick straight to Rust. Native expressions retain and calculate the 500 labels, summaries, visibility and spacing. The harness performs the same 500-label, resize and scrolling checks. This is an architectural comparison, not a faster implementation of unchanged React reconciliation. Its controls are predeclared; changing the cell count hides controls instead of inserting or removing them.

```sh
scripts/agent-tools headless --app benchmarks/apps/ink-views --hyperfine --background
```

Use `--app benchmarks/apps/ink-native-controls` for 1,000 keyed rows. It checks native-only window scrolling, row actions, prepend/reverse/delete, and controlled Japanese/emoji input across recycling. It then measures replacing all 1,000 data records while Rust updates the mounted window. `nativeScroll` includes native window materialisation and layout without a JavaScript window callback. This workload is distinct from updating all 500 mounted cells with resize; do not compare their totals as equivalent work. Its report omits the accumulated waterfall.

Use `--app benchmarks/apps/ink-list` to check the unchanged public `List` API. The fixture compares compiled text/layout rows, custom stateful React rows, standard `Row` controls and indirect React text content. It checks bounded windows, row actions, local state, prepend and native reorder anchoring. `NativeScroll` and `RowScroll` must produce no JavaScript window callbacks; `ReactScroll` and `RichScroll` exercise compatibility fallbacks. Native and React modes share layout, but React rows include a state hook and the harness waits for a window-settled acknowledgement. The fixture also measures unrelated parent updates and one-row edits in a 1,000-record dataset. It asserts zero transferred records for unrelated updates and exactly one for a single-row edit, fresh callback state for edited and visually unchanged rows, and keyed deletion. Android request dispatch during momentum scrolling requires a device regression check; the CPU harness does not run MainActivity.

The public-list fixture also scrolls 1,000-item `ReorderList` and `ConversationScreen` pages. It checks reorder actions, conversation end positioning and zero JavaScript window callbacks for both built-ins. These checks are included in whole-process time, outside the individual update samples. For a shorter iteration run, use `--iterations 50 --warmup 5`.

For quick iteration, use `scripts/agent-tools headless --app benchmarks/apps/ink-list --iterations 50 --warmup 5 --hyperfine --background`. The expanded suite takes roughly 0.42 seconds per prepared run on the development Mac. These are CPU scene measurements, not phone frame times.

| Stage | Included work |
| --- | --- |
| Input → dispatch | Native pointer-up, hit testing and obtaining the React event |
| JavaScript and transport | Queue delivery, event handling, React reconciliation, native batch conversion and receiving the commit |
| Decode | Zero for native batches; JSON deserialisation for legacy messages |
| Apply and layout | Updating the real Rust tree and rebuilding the scene |
| Input → scene | The whole path above |

`commitBytes` is null for native batches because no complete JSON document crosses the queue. Create/update properties still use JSON conversion to preserve complex-value behaviour; that work is included in JavaScript and transport.

Timings use the host's monotonic clock. JavaScript/transport includes scheduling and queue waits; it does not isolate React alone. Stage medians need not sum to the whole-update median. The binary also reports an accumulated `waterfallMs` breakdown: mount/setup, warm-up updates, measured stages, cell validation, compatibility checks, shutdown and remaining harness overhead. These add to `suiteMs`, covering asset loading through runtime shutdown. Process launch, argument parsing and final JSON serialisation/output are outside that interval. This is one instrumented run, separate from Hyperfine’s repeated whole-process measurements.

**Hyperfine measures the whole process**, including mounting, warm-up, checks, JSON output and shutdown. Do not divide that duration by 200 and call it tap latency. Use `milliseconds.inputToScene` for the mean and percentiles of individual updates. `inputToSceneSamplesMs` retains every measured duration for pooled comparisons; it is omitted from compact tool output. Use identical iteration/warm-up counts when comparing builds; inspect both whole-process time and stage timings.

Each update checks all 500 expected labels and every cell's position outside the timed interval. Additional checks exercise thumb dragging, content dragging, reversing, Activity hide/show and changing the count/scrollbar. A missed tap, wrong scene, runtime error or missing commit exits unsuccessfully. Activity reveal may publish its shell before its content; that compatibility check waits for the content commit. Timed refresh updates must produce the expected complete scene immediately.

The loop waits for each update before sending the next. It measures warmed, sequential throughput and latency, **not** idle taps or paced 60 Hz behaviour. Correctness checks are excluded from stage timings but affect the work between samples. Fonts use the host's Ink configuration; Android Japanese/emoji fallback and GPU rendering still require device checks. This fixture contains numeric labels.

The tool records the resolved QuickJS dependency's source hashes, including local patches. It also checks the C/header copies used by Cargo against those sources before benchmarking. A stale-copy error means the dependency needs rebuilding; do not accept that run as evidence for a source change. Cargo's build records are saved in `host-build.jsonl` and, for Android, `android-build.jsonl`.

Mac timings cannot establish an LP3's 16 ms budget. Use this for rapid CPU-path comparisons, then verify promising changes on the phone. Android input delivery, frame scheduling, Vulkan preparation/submission and panel response are outside this harness.

## Run the CPU harness on Android

With a connected device and an existing agent-tools reservation:

```sh
scripts/agent-tools headless --serial DEVICE_SERIAL --token RESERVATION_TOKEN \
  --ndk "$ANDROID_NDK_HOME" --iterations 100 --background
```

This prepares the same bundle on the Mac, cross-compiles the harness with `cargo ndk` for arm64/API 34, and runs it through ADB. Set `--ndk` to the installed NDK directory, or omit it when `ANDROID_NDK_HOME` is set. Diagnostic `--react-profile` modes also work. Hyperfine remains host-only.

The tool checks the reservation and thermal status, retains the binary, assets, NDK version and results on the Mac, and removes its remote scratch directory afterwards. It does not install an app or send screen taps. The reservation remains owned by the caller and must be released after device work finishes.

This runs as Android shell, outside an Activity. The driver uses the engine's existing scoped performance-core preference, as does the JavaScript runtime. Android app scheduling and rendering remain excluded; do not compare these CPU timings directly with tap-to-frame submission. Use identical iteration counts, repeated alternating builds and a quiet device for comparisons. The full 500-cell correctness checks are unchanged. The saved command reruns the tool using cached build inputs; Android scratch files are not left installed.

## Diagnose React execution

```sh
scripts/agent-tools headless --react-profile coarse --background
scripts/agent-tools headless --react-profile jsx --background
scripts/agent-tools headless --react-profile host --background
```

Run these separately. They create instrumented copies of the production bundle without editing application source, installed React packages or the engine. All normal scene checks still run. `result.json` retains every measured update's `reactProfiles`; `reactProfileMeans` and `report.md` summarise them. The prepared command remains reusable without rebuilding.

- `coarse` measures function-component execution, React's synchronous/concurrent work loops, commit, effects and the native commit call (`nativeTransferMs`). `reconcilerWorkMs` subtracts component execution from the work loop; it includes React's render-time host callbacks. `beforeCommitOverheadMs` is event handling and other work/waiting outside those loops before commit. These are elapsed times, not CPU samples.
- `jsx` also counts work units and JSX factory calls, timing one in 32 factory calls. The sampled position rotates between updates. This measures `react/jsx-runtime` element creation; it excludes evaluating its arguments, constructing the props object, and legacy `createElement` calls.
- `host` instead samples Ink's `commitUpdate` callback, including property comparison and collecting operations. It excludes other host callbacks such as insertion/removal. This mode adds a wrapper call around each update.

`componentMs` is inside `workLoopMs`, which is inside `renderMs`. JSX estimates are inside component time; host-update estimates are inside commit time. Do not add nested measurements together. `estimated…MinusClockMs` subtracts a startup calibration of back-to-back clock reads; it is only an approximate correction, not a guarantee of zero measurement overhead. Counts ending in `Calls`, `Samples` or `Units` are counts, not milliseconds.

**Use ordinary, uninstrumented runs to accept speed changes.** The diagnostic modes add timing calls, counters, metadata encoding/decoding and reporting. Diagnostic metadata follows the native commit in a separate message; waiting for it is excluded from input-to-scene timing and included in whole-process timing. Compare repeated plain/coarse/sampled runs to assess that overhead; raw phase numbers from different modes are not an exact additive waterfall. The source hooks deliberately fail if the pinned React/renderer code no longer matches, so dependency upgrades cannot silently produce an incomplete profile. Current hooks cover the function components used by this fixture, not class-component render methods.

## Repeat or compare prepared builds

`report.md` and `result.json` contain the exact direct command. Repeating it takes no compilation. Pass two saved commands to Hyperfine to compare builds:

```sh
hyperfine --warmup 2 --runs 5 --export-json comparison.json \
  'BASELINE_DIRECTORY/ink-headless --assets BASELINE_DIRECTORY/assets --iterations 200 --warmup 20' \
  'CANDIDATE_DIRECTORY/ink-headless --assets CANDIDATE_DIRECTORY/assets --iterations 200 --warmup 20'
```

Run comparisons on the same machine with other builds stopped. A short result is an iteration signal, not proof of a small improvement. Retain the corresponding stage reports and correctness outcomes.
