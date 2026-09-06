# Benchmarks

The current comparison uses three counter apps: Ink, Expo (light-template) and Light SDK. Each has a standard header, a centred count starting at zero and an **Increase** button.

## Recorded results

The [matching-counter LP3 comparison](results/matching-counter-lp3-2026-09-06.md) supplies the README figures. It includes build and runtime samples, screenshots and device cleanup checks.

## Run the Ink counter

```sh
BENCHMARK_DEVICE=<serial> ./benchmarks/measure-ink.sh
```

Set `BENCHMARK_OUTPUT` and `BUILD_BENCHMARK_OUTPUT` to new paths to keep earlier results.

## Compare all three counters

1. Overlay `apps/expo-counter` on a copy of light-template. Keep its components, hooks, utilities and locked dependencies, then build its ARM64 release APK.
2. Add `apps/light-sdk-counter` as `benchmark-counter` in a temporary Light SDK checkout.
3. Set `EXPO_COUNTER_DIR` and `LIGHT_SDK_DIR`, then run `./benchmarks/measure-builds.sh` for clean and unchanged builds.
4. Create a JSON file mapping `ink`, `expo` and `light-sdk` to their absolute release APK paths.
5. Run the comparison:

```sh
scripts/agent-tools bench --comparison /absolute/path/counters.json --serial SERIAL --background
```

The tool reserves the device, saves results and screenshots, removes its benchmark apps and restores settings. It refuses to replace existing benchmark installations. See [agent tools](../docs/agent-tools.md) to read progress and results.

To run the harness directly, set `EXPO_COUNTER_APK` and `BENCHMARK_DEVICE`, then run `bun benchmarks/measure.ts`. Set `INK_COUNTER_APK` to use an isolated build. Direct runs require a manual reservation and cleanup. No scrolling APK is needed.

## What the measurements mean

The harness alternates frameworks across 15 cold launches, five idle-memory samples and five 100-tap workloads. It uses Android activity launch timings, PSS memory, process CPU ticks and SurfaceFlinger frame intervals.

These are not measurements of time to interactive, peak memory, battery drain or GPU usage. Use the emulator to check appearance and behaviour; publish performance results from the physical LP3.
