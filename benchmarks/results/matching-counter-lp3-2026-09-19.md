# Counter comparison on Light Phone III

Measured on 19 September 2026 using fresh ARM64 release builds of Ink, Expo and Light SDK.

| Counter | Ink | Expo | Light SDK | Ink delta vs closest |
| --- | ---: | ---: | ---: | ---: |
| APK size | 2.72 MB | 28.72 MB | 10.27 MB | −7.55 MB (−73.5%) |
| Clean app build, warm caches | 1.82 s | 57.28 s | 47.10 s | −45.28 s (−96.1%) |
| Activity launch, median | 240 ms | 544 ms | 345 ms | −105 ms (−30.4%) |
| Activity launch, p95 | 266 ms | 585 ms | 370 ms | −104 ms (−28.1%) |
| Idle memory (PSS) | 16.3 MiB | 59.2 MiB | 21.3 MiB | −5.0 MiB (−23.5%) |
| CPU time for 100 taps | 930 ms | 4,510 ms | 3,660 ms | −2,730 ms (−74.6%) |

Lower is better for every metric. The closest alternative is Light SDK in every row. Deltas use the displayed values: `Ink − closest`, with percentages relative to the closest alternative. Negative values favour Ink.

## Method

The LP3 is a TLP301 running Android 14 at 1080 × 1240 and 60 Hz. Each app shows a standard Counter header, a centred Public Sans count and an Increase button. All use the same tap coordinate, (540, 760). Minor rendering differences remain.

The harness alternates framework order across 50 force-stopped launches, five idle-memory samples and five 100-tap workloads per app. Launch rounds wait 350 ms after Android reports the activity started. Idle and workload samples settle for two seconds. Thermal status was 0 at the start and end. Foreground checks passed, and initial launches and workloads reported no runtime errors.

The median averages the middle pair for the 50 launch samples. The p95 is the 48th sorted sample (nearest rank). APK sizes use decimal MB; memory uses MiB.

Activity launch comes from Android's `am start -W`; it is not time to interactive. PSS is not peak memory. CPU time is process CPU consumed during the 100-tap workload, not input latency. SurfaceFlinger evidence includes pauses between injected taps and does not measure scrolling smoothness or battery use.

## Builds

All three apps were rebuilt for this run. Ink uses commit `fae16351cac12c74ada33a247ae620556fcb5c92`, without benchmark instrumentation or the experimental native-size changes. Its APK is **2,723,596 bytes**.

Expo uses light-template with Expo 55.0.9, React Native 0.83.4, React 19.2.0, Hermes and the new architecture. Light SDK uses version 0.1.1. Both enable release minification and resource shrinking. Light SDK's minimum one-second splash delay is removed; its content-ready check remains. All APKs contain ARM64 libraries only and use development signing keys for measurement.

Clean-build times are medians of three rounds on an Apple M4 Pro. App outputs are cleaned; dependency and compiler caches remain warm. Framework order alternates between rounds. Untimed setup, downloads and warm-up builds are excluded. The CSV also records subsequent builds with no changes.

## Checks and cleanup

All 15 workload screenshots show Count: 100. The first screenshot from each framework was visually checked; the other four are pixel-identical. Every workload recorded frames.

The harness removed all three benchmark apps, disabled SurfaceFlinger collection, restored the four changed Android settings and released the device reservation. No benchmark packages remain on the LP3.

## Evidence

- [Runtime samples](matching-counter-lp3-2026-09-19.json) and [runtime log](matching-counter-lp3-2026-09-19/runtime.txt)
- [Build samples](matching-counter-lp3-2026-09-19-build.csv), [versions](matching-counter-lp3-2026-09-19/versions.json) and [APK hashes](matching-counter-lp3-2026-09-19/artifacts.json)
- Final screenshots: [Ink](matching-counter-lp3-2026-09-19/ink-counter-5.png), [Expo](matching-counter-lp3-2026-09-19/expo-counter-5.png), [Light SDK](matching-counter-lp3-2026-09-19/light-sdk-counter-5.png)
- [Screenshot checks](matching-counter-lp3-2026-09-19/screenshot-checks.json), [fixture sources](matching-counter-lp3-2026-09-19/fixture-sources.tar.gz) and [harness](matching-counter-lp3-2026-09-19/measure.ts)
- [Source hashes](matching-counter-lp3-2026-09-19/source-hashes.json) and [comparator source hashes](matching-counter-lp3-2026-09-19/comparator-source-hashes.json)
- Settings [before](matching-counter-lp3-2026-09-19/settings-before.json) and [after](matching-counter-lp3-2026-09-19/settings-after.json), plus [remaining packages](matching-counter-lp3-2026-09-19/packages-after.txt)

Local run: `bench-20260919-115426-63584e11`.
