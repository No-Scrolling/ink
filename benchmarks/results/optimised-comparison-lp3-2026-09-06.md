# Optimised Ink, Expo and Light SDK on LP3

This historical run compared Ink, Expo and Light SDK after Ink’s startup memory changes. It used six release apps: a counter and a non-virtualised 1,000-row list for each framework, alternating framework order.

## Results

| Counter | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 4.31 MB | 26.32 MB | 10.24 MB |
| Activity launch, median / p95 | 305 / 320 ms | 502 / 564 ms | 1,214 / 1,241 ms |
| Idle memory (PSS) | 18.1 MiB | 57.6 MiB | 21.7 MiB |
| CPU time for 100 taps | 1,360 ms | 4,280 ms | 3,460 ms |
| Clean app build, warm caches | 1.73 s | 58.62 s | 48.44 s |

| 1,000-row scroll | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 4.31 MB | 26.30 MB | 10.33 MB |
| Activity launch, median / p95 | 311 / 340 ms | 1,301 / 1,372 ms | 1,196 / 1,213 ms |
| Idle memory (PSS) | 25.7 MiB | 104.5 MiB | 32.1 MiB |
| CPU time for 12 swipes | 1,410 ms | 2,730 ms | 4,170 ms |
| Clean app build, warm caches | 1.68 s | 55.22 s | 42.84 s |
| Continuous-scroll frame interval, p99 | 16 ms | 16 ms | 16 ms |
| Intervals longer than 17 ms | 2 | 1 | 2 |

Recorded at 2026-09-06T03:00:34.484Z. Thermal status was 0 before and after measurement. All expected launch, memory, workload and continuous-scroll samples are present, and every workload recorded frames.

## Method

Physical Light Phone III, Android 14, ARM64 release APKs. Each app supplies 15 force-stopped activity launches, five idle-memory samples and five interaction workloads. Counter workloads inject 100 taps; scrolling workloads use six swipes in each direction. Each scrolling app also supplies three uninterrupted five-second drags. Foreground checks after launch and at memory/frame collection reject interference from other apps.

Activity launch uses `am start -W`, not instrumented time-to-interactive. Idle PSS is sampled after a two-second settling delay; it includes Ink's one-off startup collection. Process CPU time comes from `/proc`; presented-frame intervals come from SurfaceFlinger histograms. Percentiles have histogram bucket precision. Memory is MiB and APK size is decimal MB. These measurements do not establish peak memory, battery use or per-process GPU consumption.

Build measurements use three sequential rounds on an Apple M4 Pro Mac with 24 GiB RAM, macOS 27.0. Clean steps remove app/module outputs while retaining shared compiler, Gradle and dependency caches. Dependency installation and project preparation are excluded. Runtime measurement begins after timed builds finish.

## Versions and configuration

- Ink uses React and QuickJS-ng with the current uncommitted memory improvements: first-use web loading, lazy native HTTP/keyboard/glyph initialisation, full startup allocator purging, software Canvas for surrounding Android views, and reduced release metadata retention. Ink's scene still uses Vulkan.
- Expo 55.0.9, React Native 0.83.4 and React 19.2.0; Hermes and the new architecture enabled. The prepared projects use the same template snapshot and frozen dependencies as the [earlier comparison](expo-light-sdk-lp3-2026-09-06.md). Release minification and resource shrinking are enabled; native libraries are restricted to ARM64.
- Light SDK 0.1.1 uses the same prepared SDK snapshot at `3df3c24a21247e70ad59e1bc0393ac6d63840bc2` with the repository's two benchmark modules. Release minification/resource shrinking and ARM64-only packaging remain enabled.

No app fixture was changed for this run. Framework-specific screens are not pixel-identical. All 1,000 rows remain mounted in each scrolling fixture. The earlier reports remain historical records; this report supplies the replacement README figures.

## Verification and cleanup

All six APKs were checked to contain only ARM64 native libraries. The accepted run completed all expected samples without foreground-check failures; thermal status remained 0 at its start and end. Each counter separately displayed `Count: 100` after 100 injected taps: [Ink](optimised-comparison-lp3-2026-09-06/ink-counter-100.png), [Expo](optimised-comparison-lp3-2026-09-06/expo-counter-100.png), [Light SDK](optimised-comparison-lp3-2026-09-06/light-sdk-counter-100.png). Scrolling was visually checked: [Ink](optimised-comparison-lp3-2026-09-06/ink-scroll.png), [Expo](optimised-comparison-lp3-2026-09-06/expo-scroll.png), [Light SDK](optimised-comparison-lp3-2026-09-06/light-sdk-scroll.png).

The phone's stay-awake and animation settings were restored and verified identical to the saved values: [before](optimised-comparison-lp3-2026-09-06/settings-before.txt), [after](optimised-comparison-lp3-2026-09-06/settings-after.txt). All six benchmark apps were uninstalled and a package query confirmed none remained. Build outputs remain available locally. No tests were added. Diff whitespace checks passed.

These results confirm the lower idle memory under the full comparison protocol. They do not replace the broader camera, keyboard-animation and startup-purge pause checks identified in the [native memory report](native-memory-lp3-2026-09-06.md). Run-to-run changes in other frameworks' PSS should not be attributed to Ink's code changes.

## Reproduction

Prepare the Expo and Light SDK projects as described in the earlier comparison report, then set `LIGHT_SDK_DIR`, `EXPO_COUNTER_DIR` and `EXPO_SCROLL_DIR`. Run `BUILD_BENCHMARK_OUTPUT=<new-csv-path> ./benchmarks/measure-builds.sh` for all three frameworks. Set `EXPO_COUNTER_APK`, `EXPO_SCROLL_APK`, `BENCHMARK_DEVICE` and a new `BENCHMARK_OUTPUT`, then run `bun benchmarks/measure.ts`.

[Build samples](optimised-comparison-lp3-2026-09-06-build.csv), [runtime samples](optimised-comparison-lp3-2026-09-06.json), [harness and fixture hashes](optimised-comparison-lp3-2026-09-06/source-hashes.txt), [resolved Expo versions](optimised-comparison-lp3-2026-09-06/versions.json).
