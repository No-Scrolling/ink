# LP3 runtime simplification and 500-cell update measurements

27 September 2026. Physical LP3, 500 mounted changing text cells and 50 changing row gaps. The workload was not reduced or virtualised.

The isolated-tap median is now **14.55 ms**, versus **21.03 ms** in the paired comparison: an observed **30.8% reduction**. This reaches the under-16-ms median milestone, **not a consistent under-16-ms guarantee**. Tap p95 is 33.88 ms; continuous updates have an 18.82 ms median latency. Timings end at software submission, not physical panel response.

## Retained changes

- Upgrade `rquickjs` 0.12.2 → 0.14.0, moving QuickJS-ng 0.15.1 → 0.16.2. Use the upstream C small-object arena allocator and remove Ink’s 127-line Rust allocation pool and repeated allocator callbacks. The 64 MiB heap limit and 512 KiB stack limit remain.
- Use a direct ASCII grapheme iterator for measuring and drawing text. CRLF remains one grapheme; non-ASCII text retains Unicode segmentation.
- Separate action activation, haptics and scene changes. Action-only taps dispatch without drawing the unchanged scene first. A 15-tap probe now submits 15 matching updates. Silent actions still dispatch.
- Adapt UTF-8 decoding to the newer typed-array API: own decoded text before creating JavaScript objects, keeping the unsafe byte borrow narrowly scoped.

Earlier bridge, native text-host and cache optimisations remain. No custom QuickJS fork, priority change, forced clock or benchmark-specific production path is retained.

## Paired tap waterfall

Both builds contain the same changes except the runtime upgrade/allocator replacement. Order: old-a, new-a, new-b, old-b. Each installation gets one discarded warm-up and two measured three-second probes, 15 taps each. There are 60 accepted samples per build and no rejected samples. Instrumentation is identical (`INK_BRIDGE_TIMING=1`).

| Stage | Old median | New median |
| --- | ---: | ---: |
| Native input → JS dispatch | 0.32 ms | 0.30 ms |
| JavaScript, React and transport | 15.29 ms | 10.25 ms |
| Decode | 0.36 ms | 0.29 ms |
| Decode → apply | 0.06 ms | 0.05 ms |
| Apply and layout | 2.13 ms | 1.69 ms |
| Commit → frame submission | 2.45 ms | 2.05 ms |
| **Whole update** | **21.03 ms** | **14.55 ms** |
| Whole-update p95 | 35.98 ms | 33.88 ms |
| Samples below 16 ms | 6/60 | 38/60 |

Phase medians do not necessarily sum to the whole-update median. JavaScript/transport includes event handling, React, serialisation and queueing; it is not React reconciliation alone. CPU frequencies were not fixed, so these are device outcomes rather than clock-normalised instruction-speed measurements.

The four new measured-run medians were 14.65, 15.11, 14.41 and 14.43 ms. Old medians were 17.79, 32.71, 17.04 and 32.61 ms. The variation and tail latency are material.

Old build: `experiment-20260927-211240-558ff984`. New: `experiment-20260927-211635-c9af95d9`. See [sample summaries](tap-summary.json), [run provenance](tap-runs.json) and copied per-probe logs/results.

## Continuous 60 Hz workload

Each three-second run requests 180 complete updates against a 60 Hz deadline sequence. ABBA order, one warm-up plus two measured runs per installation. This compares the previous complete candidate with the new complete candidate, including ASCII and redraw changes; it does not isolate the runtime upgrade. Diagnostic JS-start markers allow request-to-submission matching.

| Metric | Previous candidate | New candidate |
| --- | ---: | ---: |
| Commits | 720 | 720 |
| Separately matched submissions | 717 | 720 |
| Median request → submission | 19.24 ms | 18.82 ms |
| p95 request → submission | 25.58 ms | 20.50 ms |
| Maximum | 38.29 ms | 27.73 ms |
| Samples below 16 ms | 1 | 7 |

All four measured new runs submitted all 180 updates, roughly 60.1 submissions/second. Throughput at 60 Hz does not imply latency below one refresh interval. At 60 Hz the interval is 16.67 ms; the stricter 16 ms goal leaves some margin.

Old build: `experiment-20260927-202145-8ac2d1f1`. New: `experiment-20260927-211841-3595bd77`. See [summary](paced-summary.json) and [runs](paced-comparison.json).

Separate short scheduler traces found:

| Stage | Previous wall / on-CPU | New wall / on-CPU | Previous / new weighted clock |
| --- | ---: | ---: | ---: |
| JS request → message ready | 13.20 / 13.20 ms | 12.65 / 12.65 ms | 1,583 / 1,255 MHz |
| Message ready → submission | 6.36 / 5.48 ms | 6.47 / 5.71 ms | 1,651 / 1,229 MHz |

These are separate diagnostic captures, not pooled ABBA medians. They indicate predominantly active execution rather than long scheduling waits, and show lower observed clocks in the new build. They do not establish an energy saving or isolate causation. Both relevant threads were in Android’s top-app CPU/cpuset group. Filtered evidence: [new](paced-new-cpu.json), [previous](paced-old-cpu.json). Probe IDs: `probe-20260927-212215-4ae9ee2c`, `probe-20260927-213401-cd8d2388`.

## Scrollbar and compatibility

| Interaction | Frames | Native frame p95 | Maximum |
| --- | ---: | ---: | ---: |
| Drag the thumb | 161 | 1.39 ms | 3.08 ms |
| Drag the content | 189 | 1.92 ms | 3.16 ms |

Both three-second probes reported zero cache misses. Scrolling stays native without invoking React for every movement. These are native frame-work measurements, not finger-to-panel latency. Resizing/reversing populated content, Hide/Show and reducing 500 cells to 100 were visually checked; the scrollbar disappeared when content no longer needed it. See [scroll evidence](scroll.json) and `updates-*.png`.

Validation completed:

- Workspace `bun run check`; TypeScript checks for template and paced fixture; Cargo checks for ink-core, ink-runtime, ink-renderer-vulkan and ink-compiler; `git diff --check`.
- Actual Ink AppRuntime with the upgraded engine produced identical React compatibility fixture output to the existing comparison, including commits, callbacks, refs, links, errors and unmounts. See [output](actual-runtime-compat.json).
- Existing scene harness checked 24 React commits against matching text, quads and scrollbar output. Pointer checks confirmed action dispatch and haptics independently of redraw, including silent actions.
- All 16,384 ASCII two-byte combinations matched Unicode grapheme segmentation, plus CRLF and non-ASCII examples.
- Runtime checks covered UTF-8 streaming splits, Japanese, combining marks, emoji, invalid UTF-8, detached/resized buffers, timers, microtasks and cancellation. Upstream language/closure/loop checks passed, alongside getters, proxies, receiver replacement during GC and errors. Object/cycle stress passed; an 80 MiB ArrayBuffer was refused by the 64 MiB limit and the runtime remained usable.
- Normal uninstrumented template on the visible emulator: navigation, Japanese wrapping, mixed scripts and emoji, combining marks, skin tones, ZWJ families, flags, keycaps and scrolling.
- Normal template on LP3: native keyboard entry of `Ink500` and Enter submission into search results. Screenshots included.

These are targeted compatibility checks and smoke tests, not exhaustive coverage of every component. Temporary harnesses remain outside production source. Template build: `experiment-20260927-211925-fff8ea69`; scrollbar build: `experiment-20260927-212636-acc5e6ec`.

## Size trade-off

| Instrumented update fixture | Previous runtime | New runtime | Change |
| --- | ---: | ---: | ---: |
| APK | 2,707,320 B | 2,742,720 B | +35,400 B (+1.31%) |
| Native shared library | 2,526,056 B | 2,561,456 B | +35,400 B |

This is a measured speed/simplicity trade-off, not a size reduction. These describe the update fixture, not a fresh counter-app size measurement. Build manifests with source/artifact hashes are included.

## Next investigations

1. **JavaScript work and allocation per update.** It remains the largest stage. Reduce work between React and native without skipping updates or changing component semantics. Compare sustained latency as well as taps.
2. **Native apply/layout and frame preparation.** The continuous trace still spends about 5.7 ms on native CPU work. Profile invalidation scope, tree traversal and transient allocation, preserving inherited layout and scrollbar size changes.
3. **Tail behaviour.** Separate GC, clock transitions and scheduling effects before choosing a fix. Tap p95 remains above 16 ms.

The scrollbar is a lower priority than these paths. Do not use forced clocks or increased priority to disguise work.

An initial, pre-change native sample profile helped choose this iteration: interpreter dispatch accounted for about 63% of sampled JS-thread cycles; Unicode grapheme iteration for about 13% of main-thread samples. Those figures are not a final-build profile. [Functions](functions.json) and [provenance](profile-provenance.json) record attribution; the packed library’s text section was verified against its symbol file.

Rejected trials included custom property-bytecode caching, glyph-cache/buffer variants, QuickJS size optimisation and higher JS priority. None had a convincing enough measured benefit to retain. See [exploration](exploratory-runs.json) and [priority comparison](paced-priority.json). Simpler upstream allocation won over the custom engine fork.
