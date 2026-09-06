# Matching counters on Light Phone III

Measured 2026-09-06T18:06:39.199Z. Ink, Expo and Light SDK ARM64 release counters were measured in one interleaved physical LP3 run after their visible components were aligned. Scrolling fixtures are no longer part of the comparison.

| Counter | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 4.34 MB | 28.72 MB | 10.27 MB |
| Activity launch, median / p95 | 306 / 332 ms | 466 / 587 ms | 1,213 / 1,231 ms |
| Idle memory (PSS) | 17.9 MiB | 59.4 MiB | 21.7 MiB |
| CPU time for 100 taps | 1,420 ms | 4,280 ms | 3,490 ms |
| Clean app build, warm caches | 1.78 s | 58.40 s | 47.87 s |

## Matching components

Each app presents a standard header titled Counter, a centred Public Sans count starting at zero, and an Increase action. Expo uses light-template's Header and StyledButton. Light SDK uses LightTopBar and LightText with lightClickable, matching typography through LightTheme and a bundled copy of the same Public Sans font. Ink uses Screen, Text and Button. Small differences in native header geometry, line metrics and rasterisation remain; the fixtures are not pixel-identical. All use the same input coordinate (540, 760).

## Method and limits

Physical TLP301, Android 14, Physical size: 1080x1240, 60 Hz. Each app supplied 15 force-stopped activity launches, five idle-memory samples after a two-second settling delay and five 100-tap workloads. Framework order alternates. Activity timing comes from Android am start -W, CPU from process ticks, memory from dumpsys meminfo and presentation intervals from SurfaceFlinger. The harness checks foreground apps and runtime errors. Every workload recorded frames. Thermal status was 0 before and after the run.

Values are medians unless labelled p95. APK size is decimal MB; PSS is MiB. This does not measure time to interactive, peak memory, battery consumption or GPU use. Counter frame intervals include input-command gaps and are not scrolling-smoothness results.

Build timings use three sequential rounds on an Apple M4 Pro with 24 GiB RAM, macOS 27.0. Clean steps remove app outputs but keep shared compiler, Gradle and dependency caches. Dependency installation, preparation and warm-up are excluded. Runtime measurement started after all timed builds finished. No-change build samples are retained in the CSV.

## Versions and evidence

Ink uses React and QuickJS-ng from the working checkout. Expo 55.0.9, React Native 0.83.4 and React 19.2.0 use Hermes and the new architecture, with release minification and resource shrinking. Light SDK uses the same prepared 0.1.1 snapshot as the earlier comparison, with the updated counter fixture, release minification and resource shrinking. All three APKs contain ARM64 native libraries only. APKs use development signing keys for local measurement.

[Runtime samples](matching-counter-lp3-2026-09-06.json), [build samples](matching-counter-lp3-2026-09-06-build.csv), [source hashes](matching-counter-lp3-2026-09-06/source-hashes.txt), [fixture source archive](matching-counter-lp3-2026-09-06/fixture-sources.tar.gz), [APK hashes](matching-counter-lp3-2026-09-06/artifacts.json), [resolved Expo versions](matching-counter-lp3-2026-09-06/versions.json), [runtime log](matching-counter-lp3-2026-09-06/runtime.log).

## Verification and cleanup

All 15 workload screenshots were checked with OCR and displayed Count: 100. Final workload screenshots: [Ink](matching-counter-lp3-2026-09-06/ink-counter-5.png), [Expo](matching-counter-lp3-2026-09-06/expo-counter-5.png), [Light SDK](matching-counter-lp3-2026-09-06/light-sdk-counter-5.png).

The benchmark tool removed all three apps from the LP3. A final package query found no benchmark packages. The four changed Android settings were restored and verified against their original values, and SurfaceFlinger collection was disabled. [Before](matching-counter-lp3-2026-09-06/settings-before.json), [after](matching-counter-lp3-2026-09-06/settings-after.json), [package check](matching-counter-lp3-2026-09-06/packages-after.txt).

No tests were written. Harness bundling, shell syntax and diff whitespace checks passed. This report replaces the headline counter figures; earlier reports remain historical.
