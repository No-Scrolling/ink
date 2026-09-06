# Benchmarks

The counter and 1,000-row scrolling apps use React with Ink's QuickJS-ng runtime. The scrolling fixture deliberately renders all rows to keep its workload comparable with the earlier fixture; it does not use list virtualisation.

## Recorded results

The [current React counter and scrolling results](results/ink-react-lp3-2026-09-06.md) measure Ink on a physical Light Phone III. They include the raw runtime samples and host build timings.

The [current Expo and Light SDK results](results/expo-light-sdk-lp3-2026-09-06.md) rerun the comparison fixtures on the same LP3, with ARM64-only release APKs. Ink's measurements were collected earlier in the same session.

The [runtime comparison](results/runtime-lp3.md) and [QuickJS-ng / Hermes comparison](results/runtime-engines-lp3.md) record the JavaScript runtime experiments on a physical Light Phone III. Their reports describe the measured builds and limitations.

Apart from the current results and runtime experiments above, files in `results/`, along with `baselines/`, `budgets.json` and `budgets-lp3.json`, are historical measurements or budgets from the removed declarative engine. They do not describe current React app performance. The `ink-updates` results measured that engine's state-to-scene path; its fixture and instrumentation harness have been removed. Check out the recorded source revisions to reproduce those measurements.

## Run current benchmarks

Install workspace dependencies with `bun install`, build the current Ink CLI, and connect a Light Phone III through ADB:

```bash
export BENCHMARK_DEVICE=<adb-serial>
./benchmarks/measure-ink.sh
```

This builds and measures the React counter and scrolling apps. Outputs default to `results/ink.json` and `results/ink-build.csv`; copy recorded results before overwriting them. No current runtime budget has been established. Set `INK_BENCHMARK_BUDGETS` to a budget for your runtime and device to enable verification.

Set `BENCHMARK_OUTPUT` and `BUILD_BENCHMARK_OUTPUT` to preserve a named run. The build harness uses this checkout's `scripts/ink`. Its clean step removes each app's Android build outputs; shared Cargo, Gradle and dependency caches remain warm. These timings do not represent a first installation of the toolchain or a build with empty caches.

For an existing measurement:

```bash
INK_BENCHMARK_BUDGETS=/path/to/budget.json \
bun benchmarks/verify.ts /path/to/runtime.json /path/to/build.csv
```

To compare Expo and Light SDK, prepare the Expo fixtures and add the Light SDK fixtures as temporary Gradle modules named `benchmark-counter` and `benchmark-scroll`. Set `LIGHT_SDK_DIR`, `EXPO_COUNTER_DIR`, `EXPO_SCROLL_DIR`, `EXPO_COUNTER_APK` and `EXPO_SCROLL_APK` to those projects and release APKs. Then run `./benchmarks/measure-builds.sh` and `bun benchmarks/measure.ts`.

Use `BENCHMARK_STACKS` to select `ink`, `expo` and `light-sdk`; use `BUILD_BENCHMARK_STACKS` with `ink`, `expo` and `light` for build measurements.

The harness alternates framework order. It measures cold process starts with Android `am start -W`, memory with `dumpsys meminfo`, process CPU time from `/proc`, and frame intervals through SurfaceFlinger. Workloads use 100 counter taps, twelve scrolling swipes and uninterrupted five-second drags. These measurements do not measure battery drain or per-process GPU use.

Uninstall benchmark apps when the run finishes.
