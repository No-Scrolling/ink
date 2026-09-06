# Expo and Light SDK benchmarks on Light Phone III

Fresh comparison measurements for the existing counter and 1,000-row scrolling fixtures. Ink was measured earlier in the same session; Expo and Light SDK alternate within this separate run. The README combines these current results rather than reusing the September 1 figures.

## Results

Measured on 6 September 2026 at 01:34 BST on a TLP301 running Android 14, with a 1080 × 1240 display at 60 Hz. Thermal status was 0 before and after the run. All four apps supplied the expected 15 launch, five memory and five workload samples; every workload recorded rendered frames. Each scrolling app supplied three continuous-scroll samples.

| Counter | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 4.30 MB | 26.32 MB | 10.24 MB |
| Activity launch, median / p95 | 329 / 357 ms | 432 / 558 ms | 1,187 / 1,207 ms |
| Idle memory (PSS) | 35.5 MiB | 54.7 MiB | 18.3 MiB |
| CPU time for 100 taps | 1,400 ms | 4,370 ms | 3,530 ms |
| Clean app build, warm caches | 1.67 s | 56.22 s | 49.04 s |

| 1,000-row scroll | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 4.30 MB | 26.30 MB | 10.33 MB |
| Activity launch, median / p95 | 327 / 366 ms | 1,308 / 1,377 ms | 1,204 / 1,225 ms |
| Idle memory (PSS) | 45.6 MiB | 101.6 MiB | 32.5 MiB |
| CPU time for 12 swipes | 1,490 ms | 2,860 ms | 4,170 ms |
| Clean app build, warm caches | 1.74 s | 55.58 s | 45.14 s |
| Continuous-scroll frame interval, p99 | 16 ms | 16 ms | 16 ms |
| Intervals longer than 17 ms | 1 | 2 | 1 |

Ink's columns come from its earlier run, linked below. APK sizes use decimal MB; memory uses MiB. Build times are medians of three rounds.

## Builds and fixtures

- Expo 55.0.9, React Native 0.83.4 and React 19.2.0, with Hermes and the new architecture enabled. The fixtures were overlaid on a temporary copy of `light-template` at revision `5a5eaadd09251c37e244da21bc18082aacbe004f`; its frozen Bun lockfile supplied the dependencies. Android release minification and resource shrinking were enabled. Native code is restricted to ARM64.
- Light SDK 0.1.1 at revision `3df3c24a21247e70ad59e1bc0393ac6d63840bc2`, using a temporary SDK copy with the two benchmark modules included. Both release builds enable minification/resource shrinking and use an explicit benchmark signing key. The fixtures now restrict native libraries to ARM64; their previous Gradle configuration packaged multiple architectures, so the older APK sizes are not directly comparable.
- Both source checkouts were clean when copied. The benchmark source files remain in this repository. The counter and scrolling screens were manually checked on the phone before measurement. These are the existing framework-specific fixtures, not pixel-identical screens.

Build times use three sequential rounds on an Apple M4 Pro Mac with 24 GiB RAM, macOS 27.0. Each clean step removes the app/module build output while retaining dependency, Gradle and native build caches. Dependencies, prebuild/setup and initial warm-up builds are excluded. No-change Scroll builds immediately follow the corresponding clean build. Timed benchmark builds ran sequentially.

## Runtime method

The harness uses 15 force-stopped process launches, five idle-memory samples and five workloads for each app. Counter workloads inject 100 taps; scrolling uses six swipes in each direction, with all 1,000 rows rendered rather than virtualised. Three five-second continuous drags supply the separate scroll-frame statistics. Framework/scenario order alternates across rounds.

Android `am start -W` supplies activity launch timing, not time-to-interactive. PSS/RSS come from `dumpsys meminfo`, app-process CPU time from `/proc`, and presented-frame intervals from SurfaceFlinger histograms. Workload FPS includes intentional gaps between taps and swipes, so continuous drags are used for the smoothness rows. Histogram percentiles have bucket precision. No battery or per-process GPU measurement is implied.

[Raw runtime samples](expo-light-sdk-lp3-2026-09-06.json) include device information, sample counts, APK hashes and the harness revision/dirty-state marker. [Build samples](expo-light-sdk-lp3-2026-09-06-build.csv) retain every timed build. The [Ink report](ink-react-lp3-2026-09-06.md) records its earlier run and interpretation limits.

## Verification and cleanup

After timing, separate 100-tap checks displayed `Count: 100` in both counter apps. The phone's stay-awake and animation settings were verified restored to their original values. All Expo and Light SDK benchmark packages were uninstalled; both Ink packages were already absent. A final package query confirmed no benchmark apps remained on the physical LP3. Generated benchmark APK files, including remaining Ink outputs and archived benchmark variants, were deleted; source and result files were retained.

## Reproduction

Prepare temporary Expo projects by copying the template without its app, Android output or dependencies, then overlay `benchmarks/apps/expo-counter` and `benchmarks/apps/expo-scroll`. Install their frozen dependencies with Bun and run `expo prebuild --platform android --no-install`. Set `android.enableMinifyInReleaseBuilds=true` and `android.enableShrinkResourcesInReleaseBuilds=true` in the generated Gradle properties.

In a temporary Light SDK checkout, include the repository's Light SDK fixtures as `benchmark-counter` and `benchmark-scroll`. Set `LIGHT_SDK_DIR`, `EXPO_COUNTER_DIR` and `EXPO_SCROLL_DIR`, then run `BUILD_BENCHMARK_STACKS=expo,light ./benchmarks/measure-builds.sh` with a new `BUILD_BENCHMARK_OUTPUT` filename.

Set `EXPO_COUNTER_APK` and `EXPO_SCROLL_APK` to the generated release APKs, and run `BENCHMARK_STACKS=expo,light-sdk BENCHMARK_DEVICE=<lp3-serial> bun benchmarks/measure.ts` with a new `BENCHMARK_OUTPUT` filename. Preserve the same release settings and check both screens before collecting measurements.
