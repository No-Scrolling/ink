# Counter comparison on Light Phone III

Measured on 28 September 2026 using fresh ARM64 release builds of Ink, Expo and Light SDK. Values are medians unless stated.

| Counter | Ink | Expo | Light SDK | Ink delta vs closest |
| --- | ---: | ---: | ---: | ---: |
| APK size | 3.24 MB | 28.72 MB | 10.27 MB | −7.03 MB (−68.5%) |
| Clean app build, warm caches | 2.03 s | 61.47 s | 50.04 s | −48.01 s (−95.9%) |
| Activity launch, median | 245 ms | 536.5 ms | 349 ms | −104 ms (−29.8%) |
| Activity launch, p95 | 268 ms | 597 ms | 378 ms | −110 ms (−29.1%) |
| Idle memory (PSS) | 16.7 MiB | 59.5 MiB | 21.7 MiB | −5.0 MiB (−23.0%) |
| CPU time for 100 taps | 720 ms | 4,270 ms | 3,540 ms | −2,820 ms (−79.7%) |

Lower is better for every metric. Light SDK is the closest alternative in each row. Deltas use the displayed values, relative to that alternative. APK sizes use decimal MB; memory uses MiB.

## Changes since the previous run

Ink's release APK is **3,242,820 bytes (3.24 MB; 3.09 MiB)**. Its native library occupies 2,719,312 bytes, stored uncompressed in the APK. JavaScript occupies 51,440 compressed bytes (162,653 uncompressed).

## Method and limits

The physical TLP301 runs Android 14 at 1080 × 1240 and 60 Hz. Each app has a Counter header, centred Public Sans count and Increase button, tapped at (540, 760). Minor rendering differences remain.

The unchanged harness alternates framework order across 50 force-stopped launches, five idle-memory samples and five 100-tap workloads per app. Launch rounds settle for 350 ms after Android reports activity startup; memory and workload samples settle for two seconds. Thermal status was 0 at both ends. Foreground and runtime-error checks passed.

The launch median averages the middle pair; p95 uses nearest rank. Android's activity launch timing is not time to interactive. PSS is not peak memory. Process CPU over 100 injected taps is not input latency. SurfaceFlinger intervals include pauses between injected taps and do not measure scrolling smoothness. Every workload recorded frames.

Build times are medians of three rounds on an Apple M4 Pro, with app outputs cleaned and dependency/compiler caches warm. Framework order alternates. Warm-up builds and downloads are excluded; unchanged-build samples are also retained. Mac build measurements ran while the physical phone comparison was active, so host ADB activity was present during build timing.

## Builds

Ink uses the current working tree based on `07476490dab504c738f2692087ecdbc329c7beb1`, including uncommitted engine changes, without benchmark instrumentation. The immutable build is `experiment-20260928-223409-efd7b183`; its source snapshot hash is recorded in [the build manifest](matching-counter-lp3-2026-09-28/ink-build.json).

Expo uses light-template's locked Expo 55.0.9, React Native 0.83.4 and React 19.2.0, with Hermes and the new architecture. Light SDK uses 0.1.2 (the previous comparison used 0.1.1). Both enable release minification/resource shrinking. Light SDK's minimum one-second splash delay is removed in the disposable build checkout; its content-ready check remains. All APKs are ARM64-only release builds signed with development keys for measurement.

## Validation and cleanup

All 15 workload screenshots show Count: 100. The first image from each app was visually inspected and OCR-confirmed; the other four per app were compared with zero differing pixels.

The runner uninstalled all three benchmark apps, disabled SurfaceFlinger collection, restored the four changed Android settings and released its device reservation. A separate package-list check also confirmed no benchmark packages remain on the LP3.

## Evidence

- [Runtime samples](matching-counter-lp3-2026-09-28/result.json) and [runtime log](matching-counter-lp3-2026-09-28/runtime.log)
- [Build samples](matching-counter-lp3-2026-09-28/build.csv), [versions](matching-counter-lp3-2026-09-28/versions.json) and [APK sizes/hashes](matching-counter-lp3-2026-09-28/apk-sizes.json)
- Final screenshots: [Ink](matching-counter-lp3-2026-09-28/ink-counter-5.png), [Expo](matching-counter-lp3-2026-09-28/expo-counter-5.png), [Light SDK](matching-counter-lp3-2026-09-28/light-sdk-counter-5.png)
- [Screenshot checks](matching-counter-lp3-2026-09-28/screenshot-checks.json), [fixture sources](matching-counter-lp3-2026-09-28/fixture-sources.tar.gz), [source hashes](matching-counter-lp3-2026-09-28/source-hashes.json) and [harness](matching-counter-lp3-2026-09-28/measure.ts)
- Settings [before](matching-counter-lp3-2026-09-28/settings-before.json) and [after](matching-counter-lp3-2026-09-28/settings-after.json), plus [remaining packages](matching-counter-lp3-2026-09-28/packages-after.txt)

Temporary Expo/Light SDK checkouts and generated Ink build directories were removed after validation. Release APKs and the immutable Ink source archive remain locally. [Comparator build configuration](matching-counter-lp3-2026-09-28/comparator-build-config.tar.gz) retains the shrink settings, Expo lockfile and Light SDK splash adjustment.

Local run: `bench-20260928-223909-bc395940`.
