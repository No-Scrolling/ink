# Automatic list updates: 23 October 2026

Existing `List` applications now use persistent native storage and cooperative row preparation automatically where eligible. No source object, component change or explicit yielding is required. The measured benefit is input responsiveness during large refreshes. Total refresh time and sparse edit latency increased in this iteration.

## Before and after

These are release measurements of identical ordinary `List`, `Row` and `TextInput` application code. The baseline is the frozen, dirty source-backed implementation before automatic defaults, based on `dfef70ed9f2d5564914c868ff332eaa98c288b83`. It includes the earlier opt-in source work. This comparison isolates the automatic defaults; it is not a comparison with the clean commit or the older literature baseline.

| Operation | Before median | After median | Before p95 | After p95 |
| --- | ---: | ---: | ---: | ---: |
| Input response during a 5,000-row refresh, host | 29.164 ms | 1.613 ms | 44.192 ms | 2.959 ms |
| Complete 5,000-row refresh, host | 44.691 ms | 50.932 ms | 48.288 ms | 53.420 ms |
| Edit one of 5,000 rows, host | 0.453 ms | 0.646 ms | 0.884 ms | 1.714 ms |
| Complete 1,000-row refresh, host | 8.784 ms | 9.222 ms | 9.878 ms | 9.894 ms |
| Edit one of 1,000 rows, host | 0.285 ms | 0.330 ms | 0.456 ms | 0.557 ms |
| Complete 5,000-row refresh, Android emulator | 47.255 ms | 52.213 ms | 52.226 ms | 55.094 ms |

Median input response improved by 94.5%. All 256 candidate input acknowledgements arrived before the replacement list was ready; none did in the baseline. During these input jobs, the list itself finished in 45.901 → 53.723 ms median. The unloaded refresh row above measures a separate workload.

The full host refresh took 6.240 ms longer, and a sparse 5,000-row edit took 0.194 ms longer. Automatic yielding improves responsiveness by admitting input during preparation. It does not reduce all work or establish a general throughput improvement.

## Default behaviour

- Native lists attempt persistent storage from 1,024 rows. Smaller lists retain flat storage. Data outside the graph's limits falls back without introducing new restrictions on existing lists. Stable-key edits share unchanged records and avoid a full native version-array copy.
- Eligible compiled updates from 512 rows prepare in timer slices after an initial bounded portion. Deferral requires compiler-tagged keys, safe captures, a populated prior list and an unchanged native template. First population and template changes remain synchronous.
- Previous complete rows remain visible until the complete replacement publishes. Supersession, unmount and Activity hiding invalidate work; reveal restarts unfinished work. Row actions resolve the latest committed requested item and current index, and suppress deleted keys.
- A private native array predicate checks own data elements without invoking getters or Proxy traps. Immutable rows reuse the existing eligibility cache. Captures always receive fresh structural validation, including both original and replacement committed closures.
- Pagination is paused while rows prepare. Initial-end scrolling, current follow-end settings and atomic native commits are preserved. There is no additional runtime or Android scheduling policy change.

The budget is checked after at least 32 and at most 128 work rows, with a four-millisecond target. Individual row graphs, capture checks, array identity scans and copies can exceed that target. JavaScript array production, filtering and sorting remain application work. Bridge serialisation and native apply remain atomic. The optional source API still avoids work that cannot be inferred from ordinary replacement arrays.

## Existing workload controls

All 30 existing control workloads completed in four alternating fresh processes, with 80 measured samples per version for each workload. They do not establish a universal speed-up. Selected medians:

| Existing workload | Before | After |
| --- | ---: | ---: |
| Edit one of 10,000 simple rows | 0.591 ms | 0.964 ms |
| Prepend to 1,000 compiled rows | 4.636 ms | 5.203 ms |
| Reorder 1,000 compiled rows | 4.553 ms | 5.173 ms |
| Native scrolling, 1,000 compiled rows | 0.071 ms | 0.073 ms |
| Runtime-to-library startup | 8.494 ms | 8.644 ms |
| Controlled conversation input | 0.249 ms | 0.257 ms |

Sparse array scans and copies remain synchronous. Persistent native storage does not remove those SDK costs. An initial descriptor-heavy implementation had larger edit and reorder regressions; restoring immutable-row caching and adding the native array check reduced them before the final run. The rejected run is retained separately and is not the result above.

## Validation and measurement

The host used an Apple M4 Pro, arm64 macOS 27.2. Both versions were built before timing, then run in baseline/candidate/candidate/baseline order. Each process supplied 64 measured edits and replacements after 12 warm-ups, giving 128 samples per version and operation. Typing used 16 measured replacement jobs per version, each with 16 inputs scheduled two milliseconds apart. Median and p95 use nearest-rank quantiles of pooled samples. The harness waits for actual row content and native viewport readiness.

Input timing ends when the controlled JavaScript value is acknowledged in an applied native scene. It checks that older acknowledgements preserve the latest native draft and that the final application value matches. This is not display presentation or physical keyboard latency.

Android used the Light Phone III API 34 emulator, release APKs and the same ordinary application source in both versions. Each of four fresh app processes performed 16 replacements; two warm-ups per process were excluded, leaving 28 measured samples per version. Timing runs from input-up to submission containing the final native commit before the next input. It excludes display presentation. The fixture disables autofocus in both APKs to keep screenshots stable.

Four full-frame before/after screenshot pairs, at start and after replacement, had zero differing pixels and zero maximum channel difference at 1080 × 1240. APK size changed from 3,597,894 to 3,601,586 bytes: 3,692 additional bytes, approximately 0.10%.

Existing checks passed: 50 SDK, 16 relevant compiler, 29 runtime/data/split-web, 18 scene/navigation/image and four native-list checks, plus SDK type checking. No unit tests were added. Separate production-path probes cover capture replacement and mutation, queued results, suspended renders, pending reorder/deleted-key actions, Activity hide/reveal, getter/Proxy fallback, native storage promotion/demotion, anchors, initial end, follow end and source rebinding. The native array helper passed 42 cases and 8,192 repeated calls with zero getter, setter or Proxy-trap executions. The existing source benchmark's contracts also pass against the final candidate.

## Evidence and reproduction

Evidence is saved under `.test-output/automatic-defaults/2026-10-23/`. `baseline-source` preserves the pre-change checkout. `baseline-bundle` and `candidate-bundle` preserve release executables and compiled assets. `paired-final` contains every final raw host run and its summary; `device-raw.json`, `device-summary.json` and `pixel-comparison.json` contain device evidence. `evidence-manifest.json` records source, fixture, binary, APK and evidence hashes. The earlier `paired` directory is diagnostic only.

Prepare the ordinary fixture and build the harness:

```sh
bun test/native/prepare.ts performance
cargo build --release -p ink-test --bin ink-automatic-performance
target/release/ink-automatic-performance 64 12 1000,5000
```

For archived paired bundles:

```sh
bun test/performance/automatic-run.ts \
  --baseline .test-output/automatic-defaults/2026-10-23/baseline-bundle \
  --candidate .test-output/automatic-defaults/2026-10-23/candidate-bundle \
  --output .test-output/automatic-defaults/2026-10-23/repeat \
  --samples 64 --warmup 12 --controls
```
