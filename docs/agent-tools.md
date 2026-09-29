---
title: "Agent tools"
description: "Run experiments, benchmarks and device tasks with the repository tools."
---

Use `scripts/agent-tools` to build experiments, run benchmarks, reserve devices and inspect images or memory. Run `scripts/agent-tools --help` for commands. The wrapper uses uv and Pillow; OCR uses Apple Vision on macOS or an installed Tesseract elsewhere. Builds use the usual Ink Android toolchain.

Operations return JSON summaries with results and an evidence directory. Synchronous commands return when finished; `--background` returns a queued ID immediately. Read saved results from another shell or agent:

```sh
scripts/agent-tools list
scripts/agent-tools status OPERATION_ID
scripts/agent-tools wait OPERATION_ID --after CURSOR --timeout 30
scripts/agent-tools result OPERATION_ID --full
scripts/agent-tools cancel OPERATION_ID
scripts/agent-tools clean OPERATION_ID
```

`list` returns the five most recent operations; paginate with `--offset 5 --limit 5`. `status` includes a cursor and, after completion, a result summary. `wait` returns when that cursor changes, the operation finishes, or the timeout expires (maximum 60 seconds). Repeated waits with an unchanged cursor return a small `unchanged` response. Omit `--after` to wait for completion. Use `status ID --full` for raw status metadata, `result ID --full` for raw results, and `device status --serial SERIAL --full` for saved settings. OCR summaries show at most ten lines of 200 characters and explicitly flag truncation; full OCR output remains in the result file. Errors remain visible in compact responses.

`status.json` records progress and completion or failure. `events.jsonl` and `operation.log` retain progress history. `clean` removes only the isolated build workspace after an operation has finished; it retains APKs, hashes, reports and other evidence. Cancellation terminates the worker's process group and attempts device cleanup. If a worker is killed forcibly or a device disconnects, its reservation remains available for explicit recovery.

Results default to the ignored `.agent-tools/` directory. Set `INK_AGENT_TOOLS_DIR` to use another absolute path. Copy reports intended for publication into `benchmarks/results/`.

## Isolated builds

```sh
scripts/agent-tools experiment --ref HEAD \
  --app benchmarks/apps/ink-counter --background
scripts/agent-tools experiment --working-tree \
  --app benchmarks/apps/ink-counter --background
```

Record the returned baseline and candidate IDs. `--ref` resolves an existing commit and archives it. `--working-tree` copies tracked files and non-ignored untracked files, including local edits. Ignored build outputs and dependencies are excluded. The snapshot has per-file hashes, executable modes and a combined hash in `source.json`. `source.tar.gz` retains the actual inputs, including uncommitted changes, after workspace cleanup. Avoid editing source while a snapshot is being copied.

Dependencies are installed from `bun.lock`; builds use a shared Cargo cache in the repository's `target/` directory. Each app has its own snapshot build directories. Experiment APKs are copied out, made read-only, and identified by SHA-256 in `manifest.json`. Every installation verifies the hash again. Source workspaces may be removed without losing the APKs.

Experiments use a shared **development signing key**, cached locally so baseline and candidate APKs can replace each other. The manifest records the signing mode and tool versions. Do not distribute these APKs as production releases.

Pass feature flags with `--env`:

```sh
scripts/agent-tools experiment --working-tree --app benchmarks/apps/ink-counter \
  --env INK_SPLIT_WEB=0 --background
scripts/agent-tools experiment --working-tree --app benchmarks/apps/ink-counter \
  --env INK_MEMORY_DIAGNOSTICS=1 --background
```

Supported flags are `INK_SPLIT_WEB=0|1`, `INK_BENCHMARK=0|1`, `INK_BRIDGE_TIMING=0|1`, `INK_PRESENTATION_TIMING=0|1` and `INK_MEMORY_DIAGNOSTICS=0|1`. Only supplied flags are inherited. Instrumentation adds logging and measurement overhead; use release builds without it for production memory and performance comparisons.

`INK_BRIDGE_TIMING=1` records the input, commit and software frame-submission markers without renderer profiling or driver presentation feedback. Prefer it for short React update comparisons. Missing renderer measurements are reported as `null`, not zero. Use identical instrumentation on both builds.

`INK_PRESENTATION_TIMING=1` includes benchmark instrumentation and enables driver presentation timestamps where `VK_GOOGLE_display_timing` is supported. Logs connect native input handling, React scene revisions and presentation IDs to actual display times. Timing feedback arrives on later frames; allow extra interactions to collect the final measured frames. This measures software response, not touch sensing or physical panel response. Use ordinary `INK_BENCHMARK=1` builds for CPU profiling because presentation timing can add driver overhead. The `submit_present_ns` field measures combined submission and presentation wall time; scheduler traces separate CPU work from waiting.

`ReactDispatch` records native event dispatch, `ReactReady` records an outgoing JavaScript message, and `ReactApply` starts applying a commit on the UI thread. A ready notification isn't necessarily a commit: match these stages within an isolated interaction.

## Paired benchmarks

### Headless CPU updates

Use `scripts/agent-tools headless --hyperfine --background` for a short desktop benchmark of the real QuickJS, React, event queues and Rust layout path. No Android build or emulator is involved. Compilation happens before Hyperfine; each prepared run measures 200 updates after 20 warm-ups and checks scene correctness. `--iterations`, `--warmup` and `--runs` adjust these counts.

The result directory contains a reusable binary/bundle, input hashes, stage timings, `report.md` and `hyperfine.json`. Hyperfine measures the whole process, including startup and checks; the internal timers measure individual input-to-scene updates. Neither measures GPU work or predicts LP3 latency. See [the headless harness](../benchmarks/headless/README.md) for measurement boundaries and comparing saved builds.

Use `--app benchmarks/apps/ink-list` to compare automatic native row compilation with React compatibility through the public `List` API.

Choose `--app benchmarks/apps/ink-views` for 500 native-bound cells plus resize, or `--app benchmarks/apps/ink-native-controls` for 1,000 keyed records with native windowing, row events and recycled input checks. The latter separately reports native scrolling and full-data refresh timings.

QuickJS provenance includes the resolved dependency's source hashes and the copied C/header inputs used by Cargo. The tool rejects stale copied inputs before running the fixture, including when experimenting with an ignored local dependency fork.

For diagnostic attribution, add `--react-profile coarse`, `--react-profile jsx` or `--react-profile host`. These respectively measure React phases, sample element creation, or sample Ink's host-update callback. Source packages remain unchanged and every scene check still runs. Results include raw per-update records and means. Instrumentation adds overhead; use ordinary runs to judge speed improvements. See the harness documentation for nested timings and sampling limitations.

To run the same CPU harness directly on a reserved Android device, add `--serial SERIAL --token TOKEN --ndk /path/to/ndk`. It uses `cargo ndk`, performs the same correctness checks and removes its remote scratch files afterwards. This runs as Android shell, without an Activity or GPU; it does not replace the full-app LP3 latency check. Hyperfine is host-only. See [Android headless runs](../benchmarks/headless/README.md#run-the-cpu-harness-on-android).

### React to native update timing

The bridge fixture reuses the counter screen with a separate app ID. Build it once, reserve the emulator, and install it:

```sh
scripts/agent-tools experiment --working-tree --app benchmarks/apps/ink-bridge \
  --env INK_PRESENTATION_TIMING=1 --background
scripts/agent-tools wait EXPERIMENT_ID --timeout 60
scripts/agent-tools device acquire --serial emulator-5554 --owner bridge-profile
scripts/agent-tools device run --serial emulator-5554 --token RESERVATION_TOKEN \
  --experiment EXPERIMENT_ID --app benchmarks/apps/ink-bridge
```

Use the experiment ID and reservation token returned by those commands. Wait until the build completes before installing. With the counter foregrounded, repeat this probe for each measurement:

```sh
scripts/agent-tools probe --serial emulator-5554 --token RESERVATION_TOKEN \
  --package com.vandam.benchmark.ink.bridge --scenario bridge
```

The workload sends 15 taps over approximately three seconds, varying their timing across display phases instead of keeping a fixed relationship to 60 Hz. Each run saves `report.md`, `result.json`, `renderer.log` and `after.png` in its evidence directory. It reports sample counts, median, p95 and maximum durations and commit payload size. Samples require one dispatch, one commit and a matching submitted scene per tap; missing, ambiguous or out-of-order samples are rejected. At least five complete updates are required. This matching is intended for the isolated counter, not arbitrary apps with concurrent updates.

| Stage | What the timer includes | Source files |
| --- | --- | --- |
| Input to dispatch | Native tap handling until the event is sent to JavaScript | `platform/android/native/src/android.rs` |
| React and transport | JS event handling, React, constructing changes, native batch conversion and queue waits | `packages/ink/src/renderer.ts`, `crates/ink-runtime/src/lib.rs` |
| JSON decode | Zero for native batches; conversion happens on the JavaScript thread | `platform/android/native/src/javascript.rs`, `crates/ink-core/src/react.rs` |
| Decode to apply | Dispatch and timing-log overhead between parsing and applying | `platform/android/native/src/javascript.rs` |
| Apply and layout | Updating the native React tree, rebuilding engine nodes and layout | `crates/ink-core/src/react.rs`, `crates/ink-core/src/lib.rs` |
| Commit to frame submission | Frame scheduling, rendering and submitting the matching scene | `platform/android/app/src/main/kotlin/com/vandam/ink/MainActivity.kt`, `platform/android/native/src/android.rs`, `crates/ink-renderer-vulkan/src/compact.rs` |

Builds with Kotlin scheduling markers further split the final interval into commit-to-request, request-to-drawing-start and drawing-start-to-submission. Driver presentation feedback is matched by presentation ID when available. It arrives on later frames, so the final updates may lack that measurement; sample counts are reported separately. Driver feedback still does not measure physical panel response.

The total starts when native code receives pointer-up and ends when the matching frame has been submitted. It is not physical touch-to-display latency. Timings share Android's monotonic clock; instrumentation adds overhead. The React/transport duration does not separately attribute JavaScript computation, commit conversion or waiting. Native batches use a zero-length `ReactDecode` marker to keep the timing boundary; probes report commit bytes as unavailable because no complete JSON document crosses the queue. Compare repeated runs using identical instrumentation; do not treat a three-second sample as a precise performance guarantee. Normal release builds omit the added decode timer.

Release the reservation when finished; this also removes apps installed through that reservation:

```sh
scripts/agent-tools device release --serial emulator-5554 --token RESERVATION_TOKEN
```

### Bulk update probe

`benchmarks/apps/ink-updates` mounts 1, 10, 100 or 500 text cells. These are deliberately not virtualised: the workload measures updates across a populated tree, including cells below the viewport. Build it with `INK_PRESENTATION_TIMING=1`, reserve the device and install the experiment as above. Select a count, then run:

```sh
scripts/agent-tools probe --serial LP3LHMA531900140 --token RESERVATION_TOKEN \
  --package com.vandam.benchmark.ink.updates --scenario updates
```

The default three-second probe taps the refresh icon 15 times. `Resize on` also changes parent spacing on each update; Reverse and Hide/Show exercise reordering and React Activity. Use identical modes and warm-up runs for comparisons, then alternate baseline and candidate order. Timings combine React execution and transport; they do not isolate JSON encoding. See [the current host runtime attribution](../benchmarks/results/fresh-language-round3-2026-09-29/report.md#current-observations) for phase measurements.

For continuous throughput, build `benchmarks/apps/ink-paced` with `INK_BRIDGE_TIMING=1`. It starts with 500 mounted cells and Resize on. Run `probe` with its package `com.vandam.benchmark.ink.paced` and `--scenario paced`. The refresh action schedules 180 updates against a three-second, 60 Hz deadline sequence. Wait until the app is at rest before each run. The result reports matched submitted commits, their intervals and submission rate. These measure throughput, not individual update latency or physical display cadence; coalesced updates and long intervals remain visible in the counts and maximum interval. Keep the default three-second probe duration and initial fixture settings for comparisons.

### Renderer probes and paired comparisons

For fast iteration, use an already installed `INK_BENCHMARK=1` build on a scrollable page:

```sh
scripts/agent-tools probe --serial emulator-5554 --token RESERVATION_TOKEN \
  --package com.vandam.benchmark.ink.rendererscroll
```

The default workload takes about three seconds: one slow drag down the list and one back. `--scenario counter` instead taps the counter's Increase button five times per second. `--seconds 2..10` changes the duration. It records process CPU time, renderer timings, uploads, cache misses, a screenshot and runtime errors. It requires your existing device reservation, checks foreground/process continuity and thermal status, and rejects missing frame instrumentation. It does not install, restart or remove the app. Begin comparisons at the same position and cache state, finish host builds first, and repeat promising results with the paired benchmark below. This is a quick regression probe, not a compatibility test or evidence of a small performance win.

`--scenario scrollbar` drags the native thumb down and back at x=994, starting at y=560. Use the 500-cell update fixture at its initial scroll position so the first drag lands on the thumb. The ordinary scroll scenario drags the content instead. Check screenshots and thumb position before applying these coordinates to another screen. Renderer timings measure native processing, not finger-to-display latency.

```sh
scripts/agent-tools bench --baseline BASELINE_ID --candidate CANDIDATE_ID \
  --app benchmarks/apps/ink-counter --serial LP3LHMA531900140 \
  --rounds 3 --scenario counter --background
```

Run device operations sequentially: a second benchmark cannot take an already reserved device. Counter and scroll presets use LP3 interaction coordinates on a 1080×1240 display. Counter performs 100 taps. The renderer fixture at `benchmarks/apps/ink-scroll` opens into 500 virtualised rows with varying heights and unique procedural artwork. Scroll performs six swipes each way, then a separate five-second drag. `--scenario idle` omits interaction and works with other display sizes and apps. The tool does not change or virtualise the fixtures; inspect the source manifests to establish fixture equivalence. Finish builds before measuring emulator performance to avoid competing with the emulator for host resources.

Baseline/candidate order alternates each round. Each APK starts in a fresh process and settles for two seconds. Samples require the expected foreground app, thermal status 0 and no Ink/Android runtime errors. Foreground changes, process restarts during interaction or missing frame evidence fail the operation. Earlier accepted samples remain on disk.

`result.json` separates idle PSS/RSS from post-interaction memory, CPU and frame timings. Raw launches, meminfo, frame histograms, error logs and screenshots sit alongside it. `report.md` summarises idle medians, ranges and the difference. Fewer than three rounds, overlapping ranges or differences below 0.25 MiB are labelled inconclusive. This is a conservative heuristic, not a statistical significance test. PSS is not peak memory, and counter presentation gaps include input-command delays.

Benchmarks temporarily set Android's four global stay-awake/animation settings and restore them afterwards. The frame presets clear SurfaceFlinger TimeStats and disable collection on exit; do not run them during another profiler's collection. Installed experiment apps are removed on exit. The tool refuses to replace any app that was already installed before its reservation, to preserve its existing build and data.

## Device reservations and manual diagnosis

```sh
scripts/agent-tools device acquire --serial LP3LHMA531900140 --owner memory-investigation
scripts/agent-tools device status --serial LP3LHMA531900140
scripts/agent-tools device run --serial LP3LHMA531900140 --token RESERVATION_TOKEN \
  --experiment EXPERIMENT_ID --app benchmarks/apps/ink-counter
scripts/agent-tools device release --serial LP3LHMA531900140 --token RESERVATION_TOKEN
```

`acquire` returns a token, records the device identity and settings, and wakes/unlocks the LP3's non-secure unlock gate. It cannot unlock a PIN-protected device. Reservations use atomic directories under `${XDG_STATE_HOME:-~/.local/state}/ink-agent-tools/devices`, shared across repository checkouts. They are cooperative: direct ADB commands and other computers can bypass them. Tokens identify the reservation to release; reservations never expire automatically while a long experiment may still be running.

`device run` verifies and installs a recorded APK, launches it, and tracks the package for cleanup. Only packages installed through the tool are removed. If settings were changed externally after acquisition, release leaves those newer values alone. A failed cleanup keeps the reservation and remaining cleanup actions on disk; reconnect the device and retry `device release` with its token. Read-only `memory` inspection can run during a manual reservation.

## Inspect images

```sh
scripts/agent-tools image /absolute/path/screenshot.png --ocr
scripts/agent-tools image /absolute/path/before.png \
  --compare /absolute/path/after.png --region 0,20,1080,90 --threshold 0 --ocr
```

The region is `x,y,width,height`. Images must have matching dimensions; comparison does not resize them. Results include file hashes, bright/coloured pixel counts, mean colours, changed pixel count and bounding box, plus OCR text and confidence on macOS. `crop.png` contains the inspected region and `diff.png` marks changed pixels white. The threshold ignores channel differences at or below the chosen 0–255 value. Check the image too: OCR can miss or misread text.

### Verify a visual defect

Image previews can be misleading. Confirm the defect in the saved pixels before changing application or rendering code, changing emulator settings, or reporting a failed visual check.

1. Retain before/after screenshots and record the exact interaction between them.
2. Compare the affected region with `image BEFORE --compare AFTER --region X,Y,W,H --threshold 0 --ocr`. Check whether the changed bounds intersect the allegedly missing or damaged content. Exclude expected changes such as a counter increment from a claim about an unchanged header.
3. For missing text, run OCR on the after image too. OCR recognition supports presence; failure to recognise text alone does not prove absence. If file evidence contradicts the preview, investigate the inspection tooling rather than the app.
4. Reproduce a confirmed defect with a fresh capture. Include image paths, comparison result IDs and the observed difference in the report. Keep an unconfirmed visual suspicion separate from a verified failure.

The 28 September 2026 list investigation initially misreported disappearing headers. Saved-image comparison and OCR showed the headers intact; only the row count changed. See the [corrected evidence](../benchmarks/results/default-lists-2026-09-28/README.md#screenshot-validation-correction).

## Inspect memory

```sh
scripts/agent-tools memory --serial LP3LHMA531900140 \
  --package com.vandam.benchmark.ink.counter
```

Inspect an already running foreground app. The tool saves Android's meminfo and process-specific Ink logs. Android's app-summary PSS categories are reported separately from the latest `InkMemory` record emitted by an `INK_MEMORY_DIAGNOSTICS=1` build.

Opt-in native counters report host-node and text-node counts, raw text payload bytes, inline host-node bytes and child-ID capacity. Renderer counters report requested GPU instance-buffer capacities, retained CPU instance snapshots and font/image/system-glyph texture payload sizes. Normal releases compile out this accounting.

These counters are partial logical sizes, **not a full allocation profiler**: they omit allocator overhead, JSON-map allocations, other retained scene/cache structures, JavaScript heaps, driver allocations and pipeline memory. They overlap Android's PSS and must not be added to it. The log describes the latest rendered frame, not necessarily the instant meminfo was sampled. Missing counters are reported as unavailable, never as zero.

## Three-framework counter comparison

Use `scripts/agent-tools bench --comparison /absolute/path/counters.json --serial SERIAL --background` to run the counter-only comparison harness. The JSON object maps `ink`, `expo` and `light-sdk` to their release APK paths. The tool copies and hashes the APKs, reserves the device, runs 50 launches, five idle samples and five 100-tap workloads per app, then uninstalls its apps and restores settings. It refuses pre-existing benchmark installations. This mode uses the fixed comparison protocol rather than paired `--rounds`/`--scenario` options.

The evidence directory retains the harness, APK hashes, runtime log, raw samples, screenshots after each workload, and before/after settings. Use `wait ID --after CURSOR` and `result ID --full` as for paired benchmarks. Check the screenshots to confirm that injected taps reached Count: 100 before publishing results.
