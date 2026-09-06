# Agent tools

Run `scripts/agent-tools --help`. The wrapper uses uv to run the Python CLI and provision Pillow. Builds need the usual Ink Android toolchain, Bun and Git. OCR uses Apple's Vision framework through Swift on macOS, or an installed Tesseract elsewhere.

Operations return compact JSON summaries by default, including key results and an evidence directory when finished. Synchronous commands emit one final response; `--background` returns only a queued ID immediately. Full evidence stays on disk. Use these commands to resume inspection from another shell or agent:

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

Results default to the ignored `.agent-tools/` directory. Set `INK_AGENT_TOOLS_DIR` to use another absolute location. These files are local working evidence, not published benchmark results; copy selected reports into `benchmarks/results/` when appropriate.

## Isolated builds

```sh
scripts/agent-tools experiment --ref HEAD \
  --app benchmarks/apps/ink-counter --background
scripts/agent-tools experiment --working-tree \
  --app benchmarks/apps/ink-counter --background
```

Record the returned baseline and candidate IDs. `--ref` resolves an existing commit and archives it. `--working-tree` copies tracked files and non-ignored untracked files, including local edits. Ignored build outputs and dependencies are excluded. The snapshot has per-file hashes, executable modes and a combined hash in `source.json`. `source.tar.gz` retains the actual inputs, including uncommitted changes, after workspace cleanup. Avoid editing source while a snapshot is being copied.

Dependencies are installed from `bun.lock`; builds use a shared Cargo cache in the repository's `target/` directory. Each app has its own snapshot build directories. Experiment APKs are copied out, made read-only, and identified by SHA-256 in `manifest.json`. Every installation verifies the hash again. Source workspaces may be removed without losing the APKs.

Experiments use a shared, generated **development signing key**, not production signing credentials. The generated key is cached locally so baseline and candidate APKs can replace each other. The use of development signing and tool versions are recorded in the manifest. Do not distribute these APKs as production releases.

Only explicitly supplied Ink feature flags are inherited:

```sh
scripts/agent-tools experiment --working-tree --app benchmarks/apps/ink-counter \
  --env INK_SPLIT_WEB=0 --background
scripts/agent-tools experiment --working-tree --app benchmarks/apps/ink-counter \
  --env INK_MEMORY_DIAGNOSTICS=1 --background
```

The supported flags are `INK_SPLIT_WEB=0|1`, `INK_BENCHMARK=0|1` and `INK_MEMORY_DIAGNOSTICS=0|1`. Memory diagnostics are separate from the existing performance instrumentation. Instrumented builds perform extra accounting and logging: use them for diagnosis, and compare ordinary release builds for production memory and performance.

## Paired benchmarks

```sh
scripts/agent-tools bench --baseline BASELINE_ID --candidate CANDIDATE_ID \
  --app benchmarks/apps/ink-counter --serial LP3LHMA531900140 \
  --rounds 3 --scenario counter --background
```

Run device operations sequentially: a second benchmark cannot take an already reserved device. Counter and scroll presets use LP3 interaction coordinates on a 1080×1240 display. The active benchmark fixture is Counter, which performs 100 taps. The generic scroll preset remains available for other apps. Scroll performs six swipes each way, then a separate five-second drag. `--scenario idle` omits interaction and works with other display sizes and apps. The tool does not change or virtualise the fixtures; inspect the source manifests to establish fixture equivalence.

Baseline/candidate order alternates each round. Each APK starts in a fresh process and settles for two seconds. Samples require the expected foreground app, thermal status 0 and no reported Ink/Android runtime errors. Foreground changes, process restarts during interaction and missing frame evidence fail the operation rather than producing a successful comparison. Earlier accepted samples remain on disk.

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

## Pixel inspection and OCR

```sh
scripts/agent-tools image /absolute/path/screenshot.png --ocr
scripts/agent-tools image /absolute/path/before.png \
  --compare /absolute/path/after.png --region 0,20,1080,90 --threshold 0 --ocr
```

The region is `x,y,width,height`. Images must have matching dimensions; comparison never silently resizes them. Results include file hashes, bright/coloured pixel counts, mean colours, changed pixel count and bounding box, plus OCR text and confidence on macOS. `crop.png` preserves the inspected region and `diff.png` marks changed pixels white. The threshold ignores channel differences at or below the chosen 0–255 value. These are pixel facts, not an automatic judgement that a UI is correct. OCR can miss or misread text.

## Memory attribution

```sh
scripts/agent-tools memory --serial LP3LHMA531900140 \
  --package com.vandam.benchmark.ink.counter
```

Inspect an already running foreground app. The tool saves Android's meminfo and process-specific Ink logs. Android's app-summary PSS categories are reported separately from the latest `InkMemory` record emitted by an `INK_MEMORY_DIAGNOSTICS=1` build.

Opt-in native counters report host-node and text-node counts, raw text payload bytes, inline host-node bytes and child-ID capacity. Renderer counters report requested GPU instance-buffer capacities, retained CPU instance snapshots, font/image/system-glyph texture payload sizes and whether the image pipeline exists. Normal releases compile out this accounting.

These counters are partial logical sizes, **not a full allocation profiler**: they omit allocator overhead, JSON-map allocations, other retained scene/cache structures, JavaScript heaps, driver allocations and pipeline memory. They overlap Android's PSS and must not be added to it. The log describes the latest rendered frame, not necessarily the instant meminfo was sampled. Missing counters are reported as unavailable, never as zero.

## Three-framework counter comparison

Use `scripts/agent-tools bench --comparison /absolute/path/counters.json --serial SERIAL --background` to run the counter-only comparison harness. The JSON object maps `ink`, `expo` and `light-sdk` to their release APK paths. The tool copies and hashes the APKs, reserves the device, runs 15 launches, five idle samples and five 100-tap workloads per app, then uninstalls its apps and restores settings. It refuses pre-existing benchmark installations. This mode uses the fixed comparison protocol rather than paired `--rounds`/`--scenario` options.

The evidence directory retains the harness, APK hashes, runtime log, raw samples, screenshots after each workload, and before/after settings. Use `wait ID --after CURSOR` and `result ID --full` as for paired benchmarks. Check the screenshots to confirm that injected taps reached Count: 100 before publishing results.
