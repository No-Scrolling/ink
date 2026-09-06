# Benchmarks

The counter and 1,000-row scrolling apps use React with Ink's QuickJS-ng runtime. The scrolling fixture deliberately renders all rows to keep its workload comparable with the earlier fixture; it does not use list virtualisation.

## Recorded results

The [current Ink, Expo and Light SDK comparison](results/optimised-comparison-lp3-2026-09-06.md) reruns all six ARM64 release apps in one interleaved LP3 session after Ink's memory optimisations. It supplies the root README figures, raw runtime samples and three-round build timings. Ink's median idle PSS is **18.1 MiB for the counter and 25.7 MiB for the non-virtualised list**.

The earlier [React counter and scrolling results](results/ink-react-lp3-2026-09-06.md) and [Expo and Light SDK results](results/expo-light-sdk-lp3-2026-09-06.md) are retained as historical measurements from before this combined rerun.

A later [lazy image resources and compact native nodes comparison](results/renderer-nodes-lp3-2026-09-06.md) measures **17.84 MiB for the counter and 22.59 MiB for the non-virtualised list**. Compact nodes save approximately 3 MiB on the list; the counter difference is within measurement variation. This targeted Ink follow-up does not replace the full-framework comparison.

The [memory investigation](results/memory-lp3-2026-09-06.md) profiles eager web polyfills, garbage collection, allocator purging and list virtualisation on LP3. These exploratory variants are separate from the published comparison benchmarks.

The [optional web-code split](results/split-web-lp3-2026-09-06.md) saves another 5.16 MiB in the counter and 4.91 MiB in the unchanged non-virtualised list in the LP3 experiment. Release apps now package web code separately and load it automatically on first use. `INK_SPLIT_WEB=0` retains the previous packaging for comparison.

The subsequent [native startup memory reductions](results/native-memory-lp3-2026-09-06.md) bring median idle PSS to **18.07 MiB for the counter and 25.80 MiB for the non-virtualised list**, through deferred native initialisation, fuller startup purging and software rendering of the surrounding Android views. Ink's scene still uses Vulkan. The report includes paired measurements and remaining verification limits.

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
