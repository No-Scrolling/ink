# Counter comparison on Light Phone III

Measured on 8 September 2026 on a physical LP3. All three ARM64 release counters were measured in one interleaved run.

| Counter | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 1.95 MB | 28.72 MB | 10.27 MB |
| Clean app build, warm caches | 1.77 s | 59.66 s | 48.82 s |
| Activity launch, median / p95 | 222 / 239 ms | 437 / 580 ms | 1,203 / 1,219 ms |
| Idle memory (PSS) | 15.8 MiB | 60.6 MiB | 22.3 MiB |
| CPU time for 100 taps | 1,050 ms | 4,510 ms | 3,630 ms |

## Apps and method

Each app starts at zero with a standard Counter header, a centred Public Sans count and an Increase action. Ink uses Screen, Text and Button; Expo uses light-template's Header and StyledButton; Light SDK uses LightTopBar, LightText and lightClickable. Minor differences in geometry and rasterisation remain. All use the same tap coordinate, (540, 760).

The device is a TLP301 running Android 14, at 1080 × 1240 and 60 Hz. Each app supplied 15 force-stopped activity launches, five idle-memory samples after two seconds of settling and five 100-tap workloads. Framework order alternates between rounds. Thermal status was 0 at the start and end.

Values are medians unless labelled p95. APK size is decimal MB; memory is MiB. Launch timing comes from Android's `am start -W`, memory from `dumpsys meminfo`, CPU from process ticks and frame evidence from SurfaceFlinger. These measurements do not establish time to interactive, peak memory, battery consumption or GPU usage. Frame intervals include gaps between injected taps and do not measure scrolling smoothness.

## Builds

Build timings were measured on 8 September on an Apple M4 Pro with 24 GiB RAM and macOS 27.0. `benchmarks/measure-builds.sh` ran three rounds in alternating framework order after untimed warm-ups. Each clean build removes app build outputs but keeps dependency and shared compiler caches. Project setup and dependency installation are excluded. The raw samples also include a subsequent build with no changes.

Ink was rebuilt from the current checkout with the direct Vulkan renderer, retained text geometry, compact RELR relocations and shared native JSON parsing. Its APK is **1,947,973 bytes**. It uses an ordinary release build without benchmark instrumentation.

Expo and Light SDK reuse the unchanged release APKs built on 6 September; all runtime measurements above are new. Expo uses Expo 55.0.9, React Native 0.83.4, React 19.2.0, Hermes and the new architecture. Light SDK uses the prepared 0.1.1 snapshot. Both have release minification and resource shrinking. APKs contain ARM64 native libraries only and use development signing keys for local measurement.

## Evidence and cleanup

All 15 workload screenshots show Count: 100. The first capture for each framework was visually checked; the other four are pixel-identical to it. Every workload recorded frame evidence, and the harness completed its foreground and runtime-error checks.

The tool removed all three benchmark apps, disabled SurfaceFlinger collection and restored the four changed Android settings. The final package query is empty, and the saved before/after settings match.

- [Raw runtime samples](matching-counter-lp3-2026-09-08.json) and [runtime log](matching-counter-lp3-2026-09-08/runtime.txt)
- [Build timings](matching-counter-lp3-2026-09-08-build.csv)
- Final screenshots: [Ink](matching-counter-lp3-2026-09-08/ink-counter-5.png), [Expo](matching-counter-lp3-2026-09-08/expo-counter-5.png), [Light SDK](matching-counter-lp3-2026-09-08/light-sdk-counter-5.png)
- [Screenshot comparisons](matching-counter-lp3-2026-09-08/screenshot-checks.json) and [APK hashes](matching-counter-lp3-2026-09-08/artifacts.json)
- [Ink build manifest](matching-counter-lp3-2026-09-08/ink-build.json), [current source hashes](matching-counter-lp3-2026-09-08/source-hashes.txt), [comparator source hashes](matching-counter-lp3-2026-09-08/comparator-source-hashes.txt), [fixture sources](matching-counter-lp3-2026-09-08/fixture-sources.tar.gz) and [Expo versions](matching-counter-lp3-2026-09-08/versions.json)
- Settings [before](matching-counter-lp3-2026-09-08/settings-before.json) and [after](matching-counter-lp3-2026-09-08/settings-after.json), plus the [package check](matching-counter-lp3-2026-09-08/packages-after.txt)

Local run: `bench-20260908-110324-88ade4f6`. Ink build: `experiment-20260908-110300-b82a8056`.
