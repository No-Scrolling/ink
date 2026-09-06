# Ink React benchmarks on Light Phone III

Measured on 6 September 2026 using the current React/QuickJS-ng engine and optimised arm64 release APKs. This run covers Ink only; older Expo, Light SDK and declarative-engine results are not mixed into this comparison.

## Results

Physical Light Phone III (TLP301), Android 14, arm64-v8a, 1080 × 1240, 60 Hz. Captured at 00:57 BST on 6 September 2026 (23:57 UTC on 5 September). Android thermal status was 0 before and after measurement. Temporary keep-awake and animation settings were restored to their original values.

| Metric | Counter | 1,000-row scroll |
| --- | ---: | ---: |
| APK size | 4.30 MB | 4.30 MB |
| Activity launch, median / p95 | 329 / 357 ms | 327 / 366 ms |
| Idle memory (PSS) | 35.5 MiB | 45.6 MiB |
| CPU time per workload | 1,400 ms | 1,490 ms |
| Clean app build, warm caches | 1.67 s | 1.74 s |
| Continuous-scroll frame interval, p99 | — | 16 ms |

The Scroll no-change build median was 1.63 s. Continuous-scroll samples had median counts of 293 presented frames, 0 dropped frames, 0 janky frames and 1 interval longer than 17 ms. Percentiles come from SurfaceFlinger histogram buckets; 16 ms is a bucket value, not sub-millisecond precision.

Counter and stop/start swipe workloads include intentional gaps between updates, so their aggregate FPS and long present intervals are not treated as smoothness scores. The separate continuous drags provide that view. Sample counts and finite workload values were checked before publishing; no performance budget was applied.

## Method

The counter and 1,000-row scrolling fixtures are in `benchmarks/apps/ink-counter` and `benchmarks/apps/ink-scroll`. The scrolling app renders all rows, without virtualisation. The counter uses 100 injected taps per workload; scrolling uses six swipes in each direction. Three separate five-second drags supply continuous-scroll frame statistics.

Each app has 15 force-stopped process launches, five idle-memory samples and five workloads. Scenario order alternates between rounds. Android `am start -W` supplies activity launch time; it is not an instrumented measure of when React content becomes interactive. CPU time comes from the app process, memory is total PSS/RSS, and frame intervals come from SurfaceFlinger histograms. No battery or per-process GPU measurements are included.

Build timings are medians of three rounds on an Apple M4 Pro Mac with 24 GiB RAM, running macOS 27.0. The clean step removes the selected app's Android build outputs under `.ink/build/android`. Cargo, Gradle build caches and dependencies remain warm, and CLI/toolchain installation is excluded. These are app-output-clean builds, not builds from empty caches. The Scroll no-change build is measured immediately after its clean build.

The release build uses the benchmark debug keystore explicitly for signing; the application and native code are still built with release optimisation. Before measuring, the release counter was checked to increment and the release list was checked to render and scroll on the phone.

After the timed run, a separate sequence of 100 taps displayed `Count: 100`. This was a functional check, not another timing sample. Both temporary benchmark apps were then uninstalled.

## Corrections made before this run

- Clean builds now target each app's isolated build directory and use the checkout's CLI.
- The launcher is no longer mistaken for Luma's unlock overlay.
- R8 retains the `onJavaScriptReady` method called from Rust through JNI. An earlier attempt rendered blank screens because this callback was removed; that attempt was stopped and discarded.
- The harness checks for runtime errors after installation and rejects workloads with no recorded frames. Interrupt handling restores temporary settings between measurement samples.

## Reproduce

From the repository root, with the physical LP3 connected:

```sh
BENCHMARK_DEVICE=<lp3-serial> \
BENCHMARK_OUTPUT=benchmarks/results/ink-react-lp3-2026-09-06.json \
BUILD_BENCHMARK_OUTPUT=benchmarks/results/ink-react-lp3-2026-09-06-build.csv \
./benchmarks/measure-ink.sh
```

Use new output filenames for another run. [Raw runtime samples](ink-react-lp3-2026-09-06.json) include the environment, protocol, APK hashes and source revision/dirty-state marker. [Build samples](ink-react-lp3-2026-09-06-build.csv) contain every timed build. The APK hashes identify the actual measured binaries; the source includes uncommitted framework changes.
