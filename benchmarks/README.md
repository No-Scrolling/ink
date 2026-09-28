# Benchmarks

Ink's benchmark work has two purposes:

| Suite | Question |
| --- | --- |
| Ink stress test | Does Ink stay responsive and display the right content after repeated use? |
| Framework comparison | How does the same counter app compare across Ink, Expo and Light SDK? |

Use `scripts/agent-tools` for builds and device runs. It records build hashes and evidence, reserves the device and restores its settings. See [agent tools](../docs/agent-tools.md).

Use the emulator for behaviour checks and the physical Light Phone III for performance results.

For fast CPU-path iteration, use the [headless update harness](headless/README.md): `scripts/agent-tools headless --hyperfine --background`. It runs the real React-to-scene path and validates updates without Android. Verify promising changes on the physical phone before making device-performance claims.

## Current engine measurements

- [Momentum-scroll image fix](results/fling-images-2026-09-28/README.md): confirmed stalled requests, frame-boundary fix and emulator reproduction.

- [Incremental list updates](results/incremental-lists-2026-09-28/README.md): sparse transfer and callback correctness.

- [Automatic native lists](results/default-lists-2026-09-28/README.md): public API compatibility, native window updates and emulator checks.
- [Native controls and bindings](results/lp3-validation-2026-09-28/native-controls/README.md): current architecture, paired CPU measurements and LP3 validation.
- [Rust/JavaScript ownership](../docs/runtime-ownership.md): current boundaries and remaining opportunities.

- [QuickJS-ng 0.17.0 comparison](results/quickjs17-lp3-2026-09-27/report.md): latest engine, compatibility checks and 500-cell update measurements.
- [Runtime optimisation profile](results/runtime-arena-lp3-2026-09-27/report.md): update waterfall, continuous workloads and scrollbar timings before the 0.17.0 upgrade.
- [Native size profile](results/native-size-2026-09-27/report.md): earlier bundle-size investigation.

Superseded intermediate reports have been removed. See the [retention record](results/cleanup-2026-09-28.md). The stress and framework comparisons below cover different workloads and remain as historical baselines.

## Ink stress test

The [stress app](apps/ink-stress/README.md) repeats navigation, image-heavy scrolling, failed refresh/retry and dataset changes. It checks visible content and collects scrolling, renderer, memory and idle measurements.

The latest [LP3 before/after comparison](results/ink-optimisations-before-after-lp3-2026-09-12.md) passed all 20 stress cycles. Navigation response improved from 49.0 to 39.4 ms median and from 59.4 to 48.4 ms p95. It also records counter CPU, memory and Counter/Weather APK sizes. Brief fast-swipe startup gaps remain a deferred issue; steady scrolling results do not cover them.

Build an instrumented APK, then run it on a reserved device:

```sh
scripts/agent-tools experiment --working-tree \
  --app benchmarks/apps/ink-stress --env INK_BENCHMARK=1 --background
# After the experiment completes:
scripts/agent-tools stress --baseline EXPERIMENT_ID --serial SERIAL --background
```

Add `--candidate CANDIDATE_ID --rounds 3` to compare a framework change using identical fixture sources and flags. The runner alternates build order and retains a report, per-cycle samples, screenshots and logs. See the [stress instructions](apps/ink-stress/README.md) for measurement limits and failure evidence.

The existing [scroll fixture](apps/ink-scroll/README.md) remains available for focused shared-image profiling through `bench --scenario scroll`. It is a supporting fixture rather than a third suite.

## Framework comparison

The comparison uses three counter apps: Ink, Expo (light-template) and Light SDK. Each has a standard header, a centred count starting at zero and an **Increase** button.

The [recorded LP3 comparison](results/matching-counter-lp3-2026-09-19.md) includes build and runtime samples, screenshots and device cleanup checks.

### Prepare and run

1. Overlay `apps/expo-counter` on a copy of light-template. Keep its components, hooks, utilities and locked dependencies, then build its ARM64 release APK.
2. Add `apps/light-sdk-counter` as `benchmark-counter` in a temporary Light SDK checkout. The benchmark build automatically removes the SDK's minimum one-second splash delay from that checkout, keeping the content-ready check. Use a disposable checkout because this changes `LightActivity.kt`. Build a fresh APK rather than reusing one with the delay.
3. Set `EXPO_COUNTER_DIR` and `LIGHT_SDK_DIR`, then run `./benchmarks/measure-builds.sh` for clean and unchanged builds.
4. Create a JSON file mapping `ink`, `expo` and `light-sdk` to their absolute release APK paths.
5. Run the comparison:

```sh
scripts/agent-tools bench --comparison /absolute/path/counters.json --serial SERIAL --background
```

The tool reserves the device, saves results and screenshots, removes its benchmark apps and restores settings. It refuses to replace existing benchmark installations. The device harness measures the supplied APKs; the delay removal happens when building the Light SDK benchmark app.

### Measurement limits

The harness alternates frameworks across 50 cold launches, five idle-memory samples and five 100-tap workloads. It uses Android activity launch timings, PSS memory, process CPU ticks and SurfaceFlinger frame intervals. Build measurements include APK size and clean and unchanged build times.

These are not measurements of time to interactive, peak memory, battery drain or GPU usage. Counter frame intervals include pauses between taps and must not be interpreted as scrolling frame rate or input-to-display latency.

### Supporting scripts

`measure.ts` is the comparison harness invoked by agent-tools. `measure-builds.sh` collects build measurements. `measure-ink.sh` and `verify.ts` are older Ink-only counter helpers, not the stress test; the direct device runner requires manual reservation and cleanup. Prefer the agent-tools entry points above.

Only the latest results for each suite are retained.

Latest list update: [compiler-directed projection reuse](results/list-projection-2026-09-28/README.md), including paired CPU timings and compatibility checks.
