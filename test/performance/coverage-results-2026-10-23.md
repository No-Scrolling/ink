# Performance coverage baseline: 23 October 2026

All 21 host journeys and five renderer workloads passed in three fresh processes per group: 2,448 measured samples. Android passed 20 samples per cold-start, image-loading and resume case, plus a 120.41-second scrolling run. Physical energy remains unmeasured because no physical phone was connected.

These are new baselines. No production speed improvement or before/after comparison is claimed.

## Environment and evidence

- Host: Apple M4 Pro, arm64 macOS, Darwin 27.2.0; release Rust and production QuickJS bundles.
- Graphics: **llvmpipe software Vulkan**, LLVM 21.1.8, Mesa 26.0.0-devel. These are not hardware-GPU results.
- Android: Light Phone III AVD, `emulator-5556`, API 34, 1080 × 1240. Started with `emulator -avd Light_Phone_III -writable-system`.
- Toolchain: Bun 1.4.3; rustc 1.96.0 (`ac68faa20`). Base revision: `2d06d0837249df2e3a4b78567b48d83aab3614ce`, with benchmark changes captured in the evidence.
- Evidence: [aggregate results](../../.test-output/coverage/2026-10-23-final/result.json), [derived figures](../../.test-output/coverage/2026-10-23-final/analysis.json), [Android results](../../.test-output/coverage/2026-10-23-final/device/result.json) and [source/binary manifest](../../.test-output/coverage/2026-10-23-final/bundle.json).

Builds finished before serial measurements. Host group order alternated between rounds. Host journeys use 20 measured samples and four discarded warm-ups per process. Cold renderer samples are capped at 16 per process; sustained renderer workloads use at least 120 frames per process. Median and p95 use nearest-rank quantiles over pooled raw samples. Evidence directories are local, ignored build artefacts; retain them with this report.

## Search and structural lists

Times are milliseconds from native command/input to the final correct NativeList publication and ready materialised scene. Each cell is **median / p95**, with 60 samples per case. These endpoints exclude Android presentation. Full key order, visible content and viewport readiness were checked.

| Operation | 1,000 rows | 5,000 rows | 10,000 rows |
| --- | ---: | ---: | ---: |
| Insert at the start | 1.767 / 2.197 | 10.730 / 13.059 | 21.779 / 25.848 |
| Delete the first row | 1.748 / 2.164 | 11.034 / 13.097 | 22.137 / 27.654 |
| Reverse order | 1.702 / 2.200 | 11.506 / 14.510 | 23.137 / 27.695 |
| Change row geometry | 0.311 / 0.351 | 0.663 / 0.777 | 1.128 / 3.229 |
| Local search, unprepared | 4.876 / 5.107 | 24.088 / 24.960 | 48.661 / 50.177 |
| Local search, cache ready | 6.821 / 7.245 | 36.882 / 38.520 | 77.679 / 82.183 |

Local filtering uses ordinary application code with joined fields, lowercasing and `includes`. Cache readiness and use were observed through the real native helper. The unprepared query selects half the catalogue; the cache-ready Unicode query selects two thirds and publishes different row content. **These two rows are different workloads, not a cache speed comparison.** Timings include projection, publication and contract-inspection overhead, rather than just filtering.

Continuous SQLite search includes six scheduled native edits, 16 arrivals and cancellation of a real obsolete query. It reached the final correct scene in **51.771 ms median / 58.694 ms p95** from the start of the sequence. From the final intended keystroke, the figures were **39.771 / 46.694 ms**. Intended-input-to-acknowledgement was **0.419 / 5.380 ms** across 360 edits. Dispatch lateness and coalescing are retained in the raw results.

## Startup and navigation

| Host journey | Median ms | p95 ms | Samples |
| --- | ---: | ---: | ---: |
| Reopen 1,000 persisted rows in a fresh process | 18.922 | 19.997 | 60 |
| Route loading → failure → retry → ready → back | 1.348 | 1.644 | 60 |

Persisted startup writes through the SDK in a separate seed process that exits before reopening. Database schema, integrity, revision and complete payload were checked. The reader timing includes prepared asset loading and runtime/engine creation; filesystem caches were recently warmed and not evicted. It excludes Android process launch. Process exit guarantees store-worker teardown; this is not a general DataHost worker-join guarantee.

The route journey uses a genuine SQLite error and retry, verifies loading scenes, retained home state and an empty root back stack. The reported duration sums three timed stages and excludes intervening verification work. Its median retry-to-ready stage was 0.653 ms and back-to-home stage was 0.188 ms. It measures the host router and resource lifecycle.

## Text, rendering and uploads

CPU time ends at submission. Completed-render wall time also includes waiting for the frame and copying offscreen readback pixels. Both columns show **median / p95 milliseconds**. GPU timestamps in the raw data include the render/readback command buffer and exclude the preceding upload command buffer.

| Workload | CPU submission ms | Completed render ms | Samples |
| --- | ---: | ---: | ---: |
| Fresh multilingual glyph cache | 0.144 / 0.181 | 4.581 / 4.879 | 48 |
| Multilingual atlas turnover | 0.911 / 1.091 | 2.312 / 2.560 | 60 |
| Sustained 1080 × 1240 uploads | 0.084 / 0.095 | 2.547 / 2.891 | 360 |
| Sustained changing-size uploads | 0.044 / 0.095 | 1.589 / 2.740 | 360 |
| Artwork arrivals, scrolling and turnover | 0.045 / 0.140 | 1.257 / 1.475 | 360 |

Every frame passed its workload's colour, clipping or cache-consistency checks. Saved endpoints were reloaded and checked numerically. Renderer construction, fixture generation, PNG writes and assertions are outside these timings. Cold means a fresh renderer and cache; process-wide font parsing can already be warm.

The host font covers the six Latin-script strings. Greek, Cyrillic, Arabic and CJK probes contain missing glyphs and are explicitly reported as such. Visible tofu is not counted as script support. Atlas comparisons with fresh renderers check cache consistency, not independent shaping correctness.

The full-frame case made **360 staging allocations for 360 measured frames**, each **5,356,800 bytes**: 1,928,448,000 allocated bytes across the three rounds. These are actual Vulkan allocation-seam counters. Changing-size traces also retain the staging capacity and transient allocation observations. Retained and in-flight capacity can refer to the same buffer and must not be added.

## Android media and lifecycle

| Observation | Median ms | p95 ms | Samples |
| --- | ---: | ---: | ---: |
| Process start → asset-screen content submitted | 902.1 | 1,153.8 | 20 |
| Process start → multilingual content submitted | 635.4 | 825.9 | 20 |
| Asset image: fixture marker → content submitted | 27.635 | 61.289 | 20 |
| Managed file image: marker → content submitted | 21.982 | 35.748 | 20 |
| Local HTTP image: marker → content submitted | 84.380 | 100.386 | 20 |
| After backgrounding: marker → content submitted | 20.674 | 62.886 | 20 |

Cold cases restart the process and retain OS/filesystem caches. Process start has 10 ms tick quantisation, with a measured boottime/monotonic clock offset. Image markers occur before the requested React change; managed-file preparation and independent file validation finish before that marker. HTTP includes a deliberate 60 ms server delay and emulator transport. Resume starts at the fixture marker after the Android launch command; it is not pure `onResume` latency.

All images used the real encoded PNG loaders, with verified request provenance, actual 512 × 512 / 1 MiB native decodes, submitted image revisions and saved-pixel identities. Submitted frames do not establish physical display presentation.

The Android multilingual screen produced six separate non-blank script regions. Its whole-frame ordinary font cache contained 57 fallback entries and three missing entries. These are character-and-size entries, not independent per-script coverage or shaping proof.

The controlled traversal verified **16 MiB** of distinct submitted artwork and a fresh decode/upload of the original URI on revisit. During **120.41 seconds and 181 swipes**, the sustained phase completed **48 MiB across 48 sources** and pixel-verified **18 MiB across 18 submitted artwork identities**. Both actual byte totals exceeded the 8 MiB cache target. Accepted native arrivals overlapped the interval between native input markers; server response readiness was not counted as a completed image.

Independent pixel review found one gallery checkpoint clipped by 17 rows at the bottom viewport inset. Its identity and full native decode remain verified. Excluding it leaves 17 distinct identities and 17 MiB, still above the cache target. The gallery oracle verifies identity and interior regions; its nominal 512-pixel dimensions do not guarantee that the complete image is visible.

## Sustained memory, CPU and idle

The Android run collected 35 process observations. PSS was **47.80 MiB before scrolling**, peaked at **48.45 MiB**, and ended at **44.16 MiB after settling and idle**. This short run does not establish the absence of leaks.

CPU use was **19.47% of one core during work** and **0.72% during the post-work idle interval**, including benchmark observation overhead. The 30.21-second idle window recorded **zero submitted frames**. Memory and CPU are emulator observations, not hardware energy measurements.

**Battery and energy were not measured.** The runner rejects emulator battery values as evidence and supports eligible physical-device counters. A physical phone is still required to validate that path; no battery-life or app-attributed joule result is claimed.

## What the baseline exposes

- At 10,000 rows, structural publication takes about 22–23 ms and the larger local-search publication takes about 78 ms. Producer work, row projection and commit application should be attributed separately before choosing the next optimisation.
- Sustained full-frame uploads allocate a fresh 5.1 MiB staging buffer per frame in this trace. Reusing large staging capacity is a concrete next experiment.
- The host and Android font observations identify coverage limits that a timing-only glyph benchmark would hide.

The SDK and fixture type checks, lint, non-perf renderer check and documentation validation passed. No unit tests were added. All benchmark correctness contracts passed in the final run. [Independent review](../../.test-output/coverage/2026-10-23-final/reviews/ink-bench-final-evidence-review-r3.md) reproduced the metrics, source/binary hashes and saved-pixel evidence, with the gallery clipping limitation above. Earlier failed attempts and smoke runs remain separate evidence and are excluded from these figures.

## Repeat the run

See [the workload catalogue](COVERAGE.md) for endpoints and limits. Use a new output directory:

```sh
bun run bench:coverage --phase build --output .test-output/coverage/next --serial emulator-5556 --samples 20
bun run bench:coverage --phase run --output .test-output/coverage/next --serial emulator-5556 --rounds 3 --samples 20 --warmup 4 --duration-seconds 120
```

The evidence records exact source, executable, APK and Vulkan-library hashes. It is not a hermetic dependency/toolchain archive.
