# LP3 counter runtime comparison

Measured 2026-09-04T20:00:29.779Z on a physical Light Phone III (TLP301), Android 14, arm64-v8a, 60 Hz.

| Metric | Native Ink | QuickJS | QuickJS + Effect |
| --- | ---: | ---: | ---: |
| APK size | 3.01 MB | 3.70 MB | 3.74 MB |
| Cold start, median | 311 ms | 309 ms | 324 ms |
| Cold start, p95 | 321 ms | 340 ms | 358 ms |
| Idle memory (PSS) | 22.76 MiB | 24.23 MiB | 24.75 MiB |
| CPU for 100 taps | 870 ms | 870 ms | 1130 ms |
| State/UI update, median | 0.274 ms | 0.294 ms | 2.139 ms |
| State/UI update, p95 | 0.531 ms | 0.563 ms | 4.581 ms |
| Script round trip, median | 0.0 µs | 20.7 µs | 1819.7 µs |
| Script round trip, p95 | 0.0 µs | 29.7 µs | 3922.9 µs |
| Runtime initialisation, median | 0.000 ms | 0.712 ms | 11.763 ms |
| Renderer CPU wall time, median | 4.713 ms | 4.581 ms | 4.804 ms |

The QuickJS + Effect APK adds 0.73 MB. Its median idle PSS is 1.98 MiB higher than native Ink, and its median cold start is 13 ms longer. The additional median state/UI update time is 1.865 ms per tap.

## Method

All three variants reuse the existing counter screen and native Vulkan renderer. The plain variant executes the increment in JavaScript; the Effect variant executes a named Effect function using Effect.runSync. Effect is pinned to 4.0.0-rc.112; rquickjs 0.12.2 vendors QuickJS-ng 0.15.1.

The existing benchmark harness alternated variant order for 15 cold starts, five idle-memory samples and five workloads of 100 taps. All 1,500 updates passed the checks for sequential counter values, rendered state changes and the embedded revision. The device thermal status was 0 before the run and 0 afterwards. Temporary animation and keep-awake settings were restored by the harness.

All APKs were instrumented arm64 release builds. Their source marker is `ec1358841dc6-1b71372b95c9`. Source and APK hashes, every timing sample and the build provenance are in [the raw results](runtime-lp3.json). An earlier run was discarded after the provenance check detected a stale baseline APK; the final build verifies the packaged native library before measurement.

## Follow-up: Effect tracing

A separate, interleaved two-variant run at 2026-09-04T20:07:51.623Z compared the same calculation wrapped in a named Effect.fn and Effect.fnUntraced. The Rust runtime, renderer and original traced script were unchanged. Both variants still use Effect.runSync.

| Metric | Named Effect.fn | Effect.fnUntraced |
| --- | ---: | ---: |
| APK size | 3.74 MB | 3.73 MB |
| Cold start, median | 324 ms | 324 ms |
| Cold start, p95 | 342 ms | 354 ms |
| Idle memory (PSS) | 23.96 MiB | 24.56 MiB |
| CPU for 100 taps | 1140 ms | 980 ms |
| State/UI update, median | 2.156 ms | 0.856 ms |
| State/UI update, p95 | 4.621 ms | 1.647 ms |
| Script round trip, median | 1843.2 µs | 542.6 µs |
| Script round trip, p95 | 4035.3 µs | 1038.2 µs |
| Runtime initialisation, median | 11.662 ms | 8.536 ms |
| Renderer CPU wall time, median | 4.909 ms | 4.966 ms |

Removing the named function's stack capture and tracing machinery reduced the median script round trip by 70.6%, from 1.843 ms to 0.543 ms. This isolates the combined cost of that machinery in this workload; it is not a profiler attribution to stack capture alone. The untraced variant still executes a generator and a synchronous effect through Effect.runSync on each call. Its remaining cost is substantial beside the plain-JavaScript result from the first run.

All 1,000 updates passed validation, with thermal status 0 before and after the run. The source marker is `ec1358841dc6-1c01f4640c4a`; [raw follow-up results](runtime-tracing-lp3.json) contain the samples and provenance. Native and plain JavaScript were not remeasured in this second run, so comparisons with those earlier numbers are descriptive rather than a four-way interleaved experiment.

Cold-start medians were identical despite the shorter runtime-initialisation interval. Idle PSS also varied between runs, and the untraced process had more threads in these samples. Do not attribute small whole-process memory or launch-time differences solely to the Effect wrapper. The used minified script shrank from 38,005 to 27,035 bytes; the plain script is 82 bytes.

For this internal hot path, fnUntraced is the better candidate. These results do not justify removing tracing from every app service, or routing every rendering operation through Effect. A useful next experiment would put asynchronous app work in Effect while retaining native layout and rendering, then measure lifecycle cancellation, responsiveness and allocation under sustained activity.

## Interpretation and limits

This is a synchronous proof of concept. Rust still owns UI state, and JavaScript executes on the calling UI thread. It measures the cost of loading the engine and used Effect bundle, crossing into JavaScript for a small operation, and returning its result to Ink. The normal release build was also verified to omit the JavaScript binding.

The state/UI update interval ends after native pointer handling and scene updates. Renderer CPU wall time is measured separately; neither metric is input-to-photon latency or GPU duration. The script interval includes the Rust/JavaScript call and number conversion. Native has no script or runtime-initialisation interval. CPU totals use coarse process accounting; small differences should not be treated as statistically significant.

The benchmark does not cover asynchronous I/O, scheduling, screen-scoped cancellation, background execution, sustained allocation pressure, scrolling under concurrent updates or battery drain. The JavaScript bundle is parsed at startup rather than precompiled to bytecode. These results support further runtime experiments, not a blanket claim about full-app performance or Effect/npm compatibility.

The experimental counter adapters and build scripts have been retired. These historical results retain the original samples and source hashes; the current framework uses its dedicated React / QuickJS-ng runtime.
