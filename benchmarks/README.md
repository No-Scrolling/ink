# Benchmarks

This benchmark compares equivalent Ink, Expo and Light SDK applications. It is designed to expose trade-offs, not manufacture a single winning score.

## Results

All sizes use decimal megabytes. Memory is Android proportional set size (PSS) and resident set size (RSS). Values are medians unless marked p95.

### Counter

| Metric | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | **3.07 MB** | 26.30 MB | 25.83 MB |
| Clean release build | **1.21 s** | 53.59 s | 46.07 s |
| Cold start median | **174 ms** | 204 ms | 1,156 ms |
| Cold start p95 | **217 ms** | 267 ms | 1,177 ms |
| Idle PSS | 26.2 MB | 61.9 MB | **25.3 MB** |
| Idle RSS | **135.7 MB** | 179.7 MB | 142.5 MB |
| Idle threads | **29** | 32 | 33 |
| Process CPU for 100 taps | **200 ms** | 400 ms | 310 ms |

Ink's APK is about 88% smaller than both alternatives. It starts 15% faster than Expo and 85% faster than Light SDK, keeps the lowest RSS and uses less process CPU for the fixed counter workload.

The APK entry breakdown is uncompressed and explains where further size work can pay off:

| Counter APK contents | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| Native libraries | 2.99 MB | 16.32 MB | 20.46 MB |
| DEX bytecode | 0.04 MB | 7.86 MB | 2.90 MB |
| Android resources | 0.01 MB | 8.49 MB | 1.45 MB |
| Assets | 0 MB | 1.70 MB | 1.02 MB |

### 1,000-row scroll view

| Metric | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | **3.07 MB** | 26.30 MB | 25.93 MB |
| Clean release build | **1.23 s** | 54.59 s | 38.81 s |
| No-op release rebuild | **1.07 s** | 9.68 s | 7.39 s |
| Cold start median | **167 ms** | 216 ms | 1,167 ms |
| Cold start p95 | **181 ms** | 402 ms | 1,182 ms |
| Idle PSS | **26.3 MB** | 110.4 MB | 40.9 MB |
| Idle RSS | **135.7 MB** | 229.1 MB | 158.7 MB |
| Idle threads | **29** | 33 | 33 |
| Process CPU for 12 swipes | **580 ms** | 850 ms | 980 ms |
| Presented frames | 252 | 183 | **255** |
| Average compositor cadence | 56.6 fps | 54.8 fps | **57.6 fps** |
| p95 presented-frame interval | 21 ms | **17 ms** | 21 ms |
| p95 compositor latency | 3 ms | **2 ms** | **2 ms** |

The 1,000 literals compile to a 9.9 KB `app.ink` asset. The stable native runtime does not relink when only application data changes, and retained visible-range layout gives Ink the lowest process CPU result of the three stacks. Emulator frame-tail data remains host-sensitive, so renderer changes are also required to pass the physical LP3 budget.

## Protocol

The six fixtures contain the same visible content, Public Sans, black and white styling, one screen and comparable geometry:

- Counter: a title, current count and one increment button.
- Scroll: a title and 1,000 text rows. Ink uses `Screen` and `Stack`, Expo uses `ScrollView`, and Light SDK uses `LightScrollView`.

The measured environment was Android Emulator 36.6.11 using the `Light_Phone_III` AVD: Android 14/API 34, arm64-v8a, 1080 × 1240 at 480 dpi and 60 Hz. The host and emulator were unchanged throughout the run. Android window, transition and animator scales were disabled.

All applications were arm64 release APKs:

- Ink used `opt-level = "z"`, fat LTO, one codegen unit and symbol stripping.
- Expo used Hermes, R8, resource shrinking and the old Expo template's dependency set.
- Light SDK used Compose, R8 and resource shrinking.
- A common debug certificate signed release code solely to make the fixtures installable. Signing does not change their runtime mode.

Dependency installation, Expo native prebuild and toolchain downloads were excluded. A clean build removes only application outputs, retaining warmed dependency and toolchain caches; Ink's stable native runtime is warmed once in the same way as React Native, Compose and Light SDK dependencies. Three build rounds alternated stack order. The no-op result immediately rebuilds the unchanged scroll application.

For runtime measurements, every sample force-stopped and started the package. One unmeasured warm-up preceded 15 alternating cold-process starts. Start-up uses Android's `am start -W` `TotalTime`, falling back to its equivalent `WaitTime` field when a device reports an unknown launch state. This is a standard launch-completion boundary rather than a framework-specific marker. These fixtures have no asynchronous application data. Memory was captured with `dumpsys meminfo` after a two-second idle period across five alternating rounds.

Each workload ran five alternating rounds:

- Counter: 100 ADB taps at the centre of the button.
- Scroll: six 350 ms upward swipes followed by six downward swipes over the same coordinates.

Scrolling also has three uninterrupted five-second drags. This removes the pauses between ADB gestures and checks the tail directly: presented-frame p95 and p99, acquire-to-present p95, intervals over 17 ms, dropped frames, janky frames and late acquisitions. On the physical LP3, both release and development builds recorded 16 ms for all three latency percentiles and zero for all four failure counts.

Process CPU is the delta of user and kernel ticks from `/proc/<pid>/stat`, converted using the device's reported `CLK_TCK` value. Frame data comes from SurfaceFlinger timestats so all three rendering technologies use the same observation point. Android `gfxinfo` was deliberately not used because Ink's Vulkan `SurfaceView` does not report its frames there.

The three-stack samples are in [raw.json](results/raw.json) and [build.csv](results/build.csv). Final Ink and LP3 validation samples are retained beside them. The fixtures and measurement harness are committed alongside the results. The comparison stacks used Light SDK `3df3c24` and Expo template `5a5eaad` on 31 August 2026.

## What the benchmark does not claim

Emulator results are good for repeatable relative comparisons, but they are not Light Phone III thermals or absolute timings. The emulator does not expose trustworthy per-process GPU utilisation or physical battery discharge. Process CPU for a fixed workload is therefore the energy proxy, while SurfaceFlinger cadence and latency are the display/GPU pipeline proxies. Reporting invented milliamp-hours would be less sound than leaving that cell out.

The counter's frame cadence is input-paced by ADB and is not a scrolling smoothness measurement. SurfaceFlinger reported no dropped frames in any scroll fixture, but that counter does not capture every missed 16.7 ms deadline. Expo also submitted materially fewer frames during the same gestures, and an interval histogram cannot count a frame that was never submitted. Read presented frames, average cadence and the interval tail together rather than treating one number as a universal smoothness score.

Before making device-specific release decisions, rerun the same protocol on a physical Light Phone III with fixed brightness, radios and temperature, and use Perfetto plus external power measurement where possible.

## Implemented performance design

- Fixed-geometry vertical lists retain one layout record and materialise only their visible range plus overscan. Scroll extent lookup is O(1), and visible layout is O(visible rows) rather than O(total rows).
- Compiler-emitted state-to-node bindings rematerialise only affected retained branches in substantial trees. Tiny trees, structural branches and native/resource completions keep cheaper conservative paths.
- Scene revisions retain GPU geometry. Scroll-only frames write one transform uniform, update a scissor and redraw a tiny scrollbar overlay.
- Choreographer is the sole presentation scheduler. It coalesces pointer movement, momentum, state changes and native completions to at most one update per display frame.
- Remote images decode on one bounded worker before their completion is scheduled for the next display frame.
- Unchanged text runs retain their generated geometry, and vertex buffers upload only their changed range.
- TSX compiles to a validated, versioned `app.ink` asset. The Rust runtime is stable across ordinary app-data changes.
- A sorted capability manifest is the sole input for optional Kotlin source sets, Cargo features, dependencies, permissions and Android components.
- The LP3 uses an opaque SurfaceView and dirty-only rendering. Unchanged applications submit no frames.
- Emulator and physical-device performance budgets are checked from retained raw samples, including continuous-scroll tail latency. One queued Vulkan frame is used on the LP3; higher optimisation levels for hot crates were measured and rejected because their size cost did not improve normal queue latency.

The remaining size opportunity is the general-purpose `wgpu`/Naga stack. Replacing it with direct Vulkan would deepen maintenance substantially, so it remains unjustified while Ink is roughly 3 MB and meets the LP3 frame budget.

## Reproducing the run

The harness expects a running emulator, prepared Expo native projects and a sibling Light SDK checkout. The Expo fixture directories are overlays for clean copies of the old Expo template; dependency installation and native prebuild happen before measurement. Add the two Light SDK fixture directories as temporary modules named `benchmark-counter` and `benchmark-scroll` in the SDK's Gradle settings, then set:

```bash
export LIGHT_SDK_DIR="$HOME/Developer/light-sdk"
export EXPO_COUNTER_DIR=/path/to/prepared/expo-counter
export EXPO_SCROLL_DIR=/path/to/prepared/expo-scroll
export EXPO_COUNTER_APK="$EXPO_COUNTER_DIR/android/app/build/outputs/apk/release/app-release.apk"
export EXPO_SCROLL_APK="$EXPO_SCROLL_DIR/android/app/build/outputs/apk/release/app-release.apk"
```

Build measurements:

```bash
./benchmarks/measure-builds.sh
```

Runtime measurements:

```bash
bun benchmarks/measure.ts
```

To build, measure and verify only the Ink fixtures without prepared Expo or Light SDK projects:

```bash
./benchmarks/measure-ink.sh
```

`BENCHMARK_STACKS` accepts a comma-separated subset of `ink`, `expo` and `light-sdk`. Build measurement uses the corresponding `BUILD_BENCHMARK_STACKS` values `ink`, `expo` and `light`. Set `BENCHMARK_DEVICE` to an ADB serial and `BENCHMARK_OUTPUT` to keep a device-specific result; otherwise the Ink-only script targets the first emulator and writes `benchmarks/results/ink.json`.

The checked Ink release limits are in [budgets.json](budgets.json). Verify an existing Ink-only result with:

```bash
bun benchmarks/verify.ts benchmarks/results/ink.json
```
