# Benchmarks

These benchmarks compare equivalent Ink, Expo, and Light SDK apps on a physical Light Phone III. Lower values are better unless the metric says otherwise.

Values are medians unless a row states otherwise. Sizes use decimal megabytes.

## Results

### Counter app

The counter app shows a title, a count, and one button.

| Metric | Ink | Expo | Light SDK | Result |
| --- | ---: | ---: | ---: | --- |
| App file size | **3.00 MB** | 32.00 MB | 25.83 MB | Ink is 88% smaller than the next-smallest app. |
| Clean release build | **1.27 s** | 36.20 s | 47.92 s | Ink builds 29 times faster than the next-fastest app. |
| Typical cold start | **292 ms** | 516 ms | 1,186 ms | Ink starts fastest. |
| Slow cold start (95th percentile) | **314 ms** | 591 ms | 1,226 ms | Ink remains fastest in slower runs. |
| Active memory while idle | **17.1 MiB** | 66.6 MiB | 18.1 MiB | Ink uses slightly less memory than Light SDK and 74% less than Expo. |
| CPU time for 100 taps | **860 ms** | 4,300 ms | 3,650 ms | Ink uses 76% less CPU time than the next-best result. |

### 1,000-row scrolling app

The scrolling app shows a title and 1,000 text rows.

| Metric | Ink | Expo | Light SDK | Result |
| --- | ---: | ---: | ---: | --- |
| App file size | **3.00 MB** | 31.98 MB | 25.93 MB | Ink is at least 88% smaller. |
| Clean release build | **1.26 s** | 36.39 s | 46.87 s | Ink builds 29 times faster than the next-fastest app. |
| Rebuild with no code changes | **1.11 s** | 9.99 s | 8.54 s | Ink rebuilds almost eight times faster than the next-fastest app. |
| Typical cold start | **285 ms** | 476 ms | 1,185 ms | Ink starts fastest. |
| Slow cold start (95th percentile) | **301 ms** | 1,373 ms | 1,233 ms | Ink remains fastest in slower runs. |
| Active memory while idle | **17.3 MiB** | 116.6 MiB | 32.3 MiB | Ink uses 46% less memory than the next-best result. |
| CPU time for 12 swipes | **890 ms** | 2,960 ms | 4,040 ms | Ink uses 70% less CPU time than the next-best result. |
| Average frame rate during swipes | 59.2 fps | 55.7 fps | **59.5 fps** | Light SDK has the highest average rate by 0.3 fps. |
| 99% of uninterrupted frame intervals are at most | **16 ms** | **16 ms** | **16 ms** | All three stay within one 60 Hz refresh. |
| Intervals longer than 17 ms during a five-second drag | **0** | 1 | **0** | Ink and Light SDK record no long intervals. |

Android reports active memory as proportional set size (PSS).

## Methodology

### Test apps

All six apps use Public Sans, black-and-white styling, one screen, and comparable geometry.

- The counter app shows a title, a count, and one increment button.
- The scrolling app shows a title and 1,000 text rows.

### Test environment

All runtime measurements ran on the same Light Phone III (`TLP301`). The device used Android 14, API 34, arm64-v8a, a 1080 × 1240 display at 480 dpi, and a 60 Hz refresh rate. Window and transition animations were disabled. Android reported thermal status 0 before and after the run.

All apps were arm64 release builds. Expo used Hermes, code shrinking, and resource shrinking. Light SDK used Compose, code shrinking, and resource shrinking. A common debug certificate signed the release builds so that the harness could install them.

Build times were measured on the same development computer. Dependency installation, native project generation, and toolchain downloads were excluded. The framework versions were Light SDK commit `3df3c24` and Expo source commit `5a5eaad`. Expo and Light SDK were measured on 31 August 2026; Ink was refreshed on 1 September 2026.

### Measurements

The harness alternated framework order between rounds to reduce ordering bias.

- Builds: three rounds with warmed dependency and toolchain caches. A clean build removed app outputs. It did not redownload dependencies.
- Startup: one warm-up followed by 15 cold process starts. The harness stopped every benchmark process before each launch. Android `am start -W` supplied the launch time.
- Memory: five samples after the app remained idle for two seconds. Android `dumpsys meminfo` supplied the values.
- Counter workload: five rounds of 100 ADB taps on each app's visible increment control.
- Scrolling workload: five rounds of six upward and six downward swipes. Each swipe lasted 350 ms.
- Continuous scrolling: three uninterrupted five-second drags.

Process CPU time comes from `/proc/<pid>/stat`. Frame measurements come from SurfaceFlinger timestats, which works with all three rendering systems.

The physical-device data is in [lp3.json](results/lp3.json) and [lp3-build.csv](results/lp3-build.csv). Emulator benchmarks remain available for automated regression checks, but their results are not included in the tables.

### Limits

These results describe one Light Phone III under the recorded conditions. They do not measure battery drain or per-process GPU use. Battery testing requires external power measurement and a longer workload.

Average frame rate and frame intervals describe different parts of scrolling performance. Read them together. A framework can report a short frame interval while producing frames less often during the same gesture.

## Reproduce the benchmark

Connect a Light Phone III through ADB. Prepare the Expo benchmark projects and add the Light SDK fixtures as temporary Gradle modules named `benchmark-counter` and `benchmark-scroll`.

Set the project locations:

```bash
export LIGHT_SDK_DIR="$HOME/Developer/light-sdk"
export EXPO_COUNTER_DIR=/path/to/prepared/expo-counter
export EXPO_SCROLL_DIR=/path/to/prepared/expo-scroll
export EXPO_COUNTER_APK="$EXPO_COUNTER_DIR/android/app/build/outputs/apk/release/app-release.apk"
export EXPO_SCROLL_APK="$EXPO_SCROLL_DIR/android/app/build/outputs/apk/release/app-release.apk"
export BENCHMARK_DEVICE=<adb-serial>
```

Measure build times:

```bash
BUILD_BENCHMARK_OUTPUT=benchmarks/results/lp3-build.csv \
./benchmarks/measure-builds.sh
```

Measure runtime performance:

```bash
BENCHMARK_OUTPUT=benchmarks/results/lp3.json \
bun benchmarks/measure.ts
```

To build, measure, and verify only Ink, run:

```bash
./benchmarks/measure-ink.sh
```

Use `BENCHMARK_STACKS` to select any combination of `ink`, `expo`, and `light-sdk`. Use `BUILD_BENCHMARK_STACKS` with `ink`, `expo`, and `light` for build measurements.

To verify an existing Ink-only result against its device budget, run:

```bash
bun benchmarks/verify.ts benchmarks/results/ink.json benchmarks/results/ink-build.csv
```

For the focused state-to-frame benchmark, use the wrapper so the APK embeds its
source revision and the harness verifies both that revision and the APK hash:

```bash
./benchmarks/measure-updates.sh
```

Set `UPDATE_BENCHMARK_ROUNDS` to change the default 25 balanced rounds. The
harness fails if a tap produces no instrumented update frame or no scene rebuild.

Uninstall the six benchmark apps when the run finishes.
