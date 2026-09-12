# Ink optimisation cleanup and before/after benchmark — LP3, 12 September 2026

All 20 stress cycles passed. The combined five optimisations improved checked navigation latency and counter CPU cost, with larger APKs and higher memory use. This is not a uniform renderer-speed improvement.

## Comparison scope

Before disables the five current optimisations: release speed optimisation, the small-allocation pool, scoped CPU preference, conditional latest-movement presentation, and avoiding unchanged system-bar colour updates. The affected tracked source is restored to HEAD `7faafe9c3f42df3196b5bad78b05e23ac29ecc68`; shared presentation logging and the current stress fixture remain identical in both builds. After uses the cleaned working-tree versions. This measures the optimisation bundle, not the effect of formatting cleanup.

The source manifests differ only in `Cargo.toml`, `crates/ink-core/src/lib.rs`, `crates/ink-runtime/src/lib.rs`, `MainActivity.kt` and `platform/android/native/src/android.rs`. The allocation/CPU helper files exist in both snapshots but are only referenced by the optimised runtime. Counter and Weather app content is identical within the APK-size comparison.

## Release APK sizes

ARM64 release APKs, without profiling flags, signed with the same development key and using isolated benchmark package names. These are APK file sizes, not compressed store downloads or installed size. Original example package/signing configuration has been restored.

| Example app | Before | After | Change |
|---|---:|---:|---:|
| Counter | 1.89 MiB | 2.57 MiB | +36.1% |
| Weather | 2.60 MiB | 3.29 MiB | +26.5% |

Exact bytes: Counter 1,983,021 → 2,699,413; Weather 2,726,235 → 3,447,883. The larger native library accounts for essentially all the increase. Weather launch/runtime measurements are excluded, as requested.

## Full stress run

One ten-cycle run per build, baseline then candidate, on LP3LHMA531900140. Identical `INK_BENCHMARK=1` and `INK_PRESENTATION_TIMING=1` flags. Content/artwork, foreground/process, thermal and error checks passed. No Perfetto trace was active.

| Metric | Before | After |
|---|---:|---:|
| Completed cycles | 10/10 | 10/10 |
| Navigation median | 48.98 ms | 39.41 ms |
| Navigation p95 | 59.40 ms | 48.42 ms |
| Navigation maximum | 62.53 ms | 48.59 ms |
| Measured navigation responses >50 ms | 17/39 | 0/39 |
| Five-second drag CPU, median | 1530 ms | 1450 ms |
| Steady-drag frame intervals, median of cycle p95 | 16 ms bucket | 16 ms bucket |
| Reported drops / steady-drag frames | 0 / 2894 | 0 / 2883 |
| Renderer frame wall time p95 | 3.407 ms | 3.502 ms |
| Relayout p95 | 0.466 ms | 0.828 ms |
| Scene preparation p95 | 0.313 ms | 0.425 ms |
| Submit/present p95 | 2.451 ms | 2.564 ms |
| Initial PSS | 20.62 MiB | 21.81 MiB |
| Final PSS | 32.42 MiB | 34.17 MiB |
| Ten-second idle frames | 0 | 0 |
| Ten-second idle CPU | 10 ms | 0 ms |

Navigation median improved by 19.5%, p95 by 18.5%, and drag CPU by 5.2% in this pair. Renderer frame p95 was slightly higher, and relayout/scene-preparation p95 were also higher; these results do not establish a general renderer CPU reduction.

Navigation uses native release → driver-reported display of the first frame at or beyond the first subsequent React commit for checked detail/tab-navigation taps. Both builds have 40 matching navigation actions, with 39 measured and the final Library return missing presentation feedback. Missing samples are not counted as passes. This measures the first committed response, excludes physical touch/panel sensing and excludes hardware-Back transitions. OCR confirms settled content but is not used as the latency clock.

The frame histogram is millisecond-bucketed: the 16 ms bucket is consistent with approximately 60 Hz. Zero reported drops during these steady drags does not establish zero fast-swipe startup gaps. Previous short tracing found those gaps, and this cleanup does not change their scheduler. Renderer timings include wall-time waits. Memory growth includes warmed caches; one run per build is not a leak or long-duration guarantee. CPU ticks have 10 ms resolution. The full stress pair is a session-specific comparison, not a statistical guarantee.

## Release Counter: three alternating rounds

Each sample starts a fresh process, settles for two seconds, measures initial PSS, then performs 100 taps. All six final screenshots show Count: 100 (600 taps total). Order: before/after, after/before, before/after. Launch figures are Android activity `TotalTime`, not full app time-to-interactive.

| Metric, median | Before | After |
|---|---:|---:|
| Activity launch | 227 ms | 220 ms |
| Initial PSS | 16.37 MiB | 17.41 MiB |
| CPU per 100 taps | 1280 ms | 820 ms |

Counter CPU falls by 35.9%; initial PSS rises by 1.04 MiB. Launch samples are before [239, 222, 227] ms and after [220, 213, 221] ms: a small observed change, not a strong general startup claim. CPU samples are before [1290, 1270, 1280] ms and after [820, 820, 830] ms. These are process CPU measurements, not battery-energy measurements.

## Cleanup and verification

Limited the new allocation and CPU-affinity helpers to parent-module visibility and tidied their formatting/module declarations. Kept the allocator bounds, CPU metadata/syscall fallbacks, thread-bound guard and scrolling-readiness checks: they handle real allocator/platform constraints and preserve behaviour. No scheduling prototype or new repository tests were retained.

`cargo check -p ink-runtime`, `cargo check -p ink-core`, `git diff --check`, Android builds and device checks passed. Current optimisation files match the measured candidate’s source hashes. Temporary example signing/package changes and the unused fixed-location Weather preference were restored. Device reservation released; managed benchmark apps removed.

## Evidence and reproduction

| Build | Experiment |
|---|---|
| beforeRelease | `experiment-20260912-203013-6d381452` |
| afterRelease | `experiment-20260912-202943-24f81f2b` |
| beforeStress | `experiment-20260912-203021-864b9cb5` |
| afterStress | `experiment-20260912-202954-3ac2090e` |

Full stress evidence: `.agent-tools/stress-20260912-203141-cfb60c4b/`.
Counter evidence: `.agent-tools/bench-20260912-204456-3640c6db/`.
Analysis scripts and build audit: `.agent-tools/cleanup-benchmark-20260912/`. The adjacent JSON includes source manifests, raw summaries, individual navigation samples and screenshot-check references.

Superseded diagnostic files have been removed. Excluded attempts: the first stress launch encountered the unlock screen before any accepted cycle; its complete rerun passed. The initial Weather measurement encountered its location-permission activity and produced no accepted sample; Weather launches were subsequently excluded at the user’s request.
