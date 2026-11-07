---
title: "Layout reuse and Android text evaluation"
description: "Measurements, correctness checks and limits of static subtree reuse."
---

## Decision

Keep static subtree layout and scene reuse. Keep Ink's existing text stack. Android text fails the host and design-export fidelity gate, including with the same Public Sans font. No Android text implementation or font removal ships.

This change reduces work for common list and row updates. It does not reduce native library size or establish a memory saving. Some cold-layout and navigation workloads regress; the measurements below include those results.

## Layout reuse

`ink-core/src/reuse.rs` retains measurements and ranges into the previous scene's primitive and hit-region buffers. It compares applied native node content, including descendants and event actions. This covers React text, prop, structural and generated-list changes without a separate invalidation protocol that could miss a commit operation.

Eligible measurements depend on width, height and scale. Position changes can reuse those measurements. Painting requires the same rectangle, clip, scrolling mode and appearance. Translated paint takes the normal path; this implementation does not shift previously rounded or clipped pixels.

Each node's content is validated once per source address and layout pass. Rust immutably borrows the native tree throughout that pass. Identity alone is insufficient because generated compositions can share owners. Subsequent passes compare content again. Parent reuse retains and rebases descendant ranges, so a later child edit can still reuse its clean siblings. Fixed-width text indices are remapped alongside copied text ranges.

Lists, editors, images, portals, canvases, captures and playback animations keep their normal layout side effects. Static descendants can reuse output. Linked text also takes normal layout because it inherits long-press behaviour from its enclosing hit region. Composer controls whose input-preservation flag changes after layout invalidate their cached hit ranges.

Measurements retain at most eight constraint pairs per node. Entries expire when their subtree is no longer measured or painted. Primitive buffers alternate between two allocations. The cache additionally retains native node snapshots and the previous primitive output; memory depends on tree size and nesting. This is a CPU optimisation with a memory trade-off.

## Correctness

Six differential tests compare incremental scenes with the original recursive layout path, with measurement and paint reuse disabled in the oracle. They cover:

- A seeded sequence of 320 text, visibility, reorder, viewport, inset, cutout, appearance and scroll changes.
- Child edits after several unchanged parent frames, with an assertion that sibling ranges actually reuse.
- Fixed-width text updates, removal and reparenting, including remapped scene indices.
- List height changes and key patches while scrolled, including reorders and deletion.
- Image arrivals, preloading and screen overlays.
- Platform video portals and paused playback progress.

Comparisons include complete scene primitives, hit regions, text-input and swipe layouts, list positions and scroll extents. Scene revision counters are normalised. The existing compiled scene suites additionally cover list anchors, held pointers, callbacks, native editing, stale React acknowledgements, navigation lifecycle and image blocking.

Run the checks with:

```sh
cargo test --release -p ink-core
bun run test scene --profile release
cargo clippy --release -p ink-core --all-targets
```

## Emulator and exports

The verification fixture is `test/fixtures/scene/reuse`: a counter, a compiled list of 2,000 rows and a detail route. Baseline and candidate builds use the same fixture. The emulator runs `emulator -avd Light_Phone_III -writable-system`, at 1080 × 1240 and 480 dpi. Captures use direct ADB commands and require three consecutive identical decoded RGBA frames.

`test/performance/layout-reuse-capture.ts` replays counter updates, list navigation, a scrollbar drag, detail navigation and back navigation. It compares saved pixels with the existing visual suite's shading tolerance: maximum channel difference 16 and at most 0.1% changed pixels, with identical alpha. It also rejects unchanged consecutive interaction states. This checks settled transition results, not every frame of a transition.

```sh
export ANDROID_HOME=$HOME/Library/Android/sdk
bun install --cwd test/visual
# Build and install the baseline fixture, then capture it.
bun test/performance/layout-reuse-capture.ts baseline .test-output/reuse-evidence
# Build and install the candidate fixture, then compare it.
bun test/performance/layout-reuse-capture.ts candidate .test-output/reuse-evidence
```

Route exports use the production `ink export` path with frozen fixture state. Each frame remains 1080 × 1240 without scaling, padding or borders. Counter, list and detail exports have zero changed pixels against the baseline.

## Android text gate

Android's [TextRunShaper](https://developer.android.com/reference/android/graphics/text/TextRunShaper), [MeasuredText](https://developer.android.com/reference/android/graphics/text/MeasuredText) and [LineBreaker](https://developer.android.com/reference/android/graphics/text/LineBreaker) provide the required platform primitives. `test/performance/AndroidTextProbe.java` exercises them on the API 34 emulator. It uses Ink's Public Sans file, fractional unhinted metrics, and an Android em size of `46 × 2000 / 2350` to match ab_glyph's ascent-to-descent scale. It also measures Android's default typeface.

The host probe uses the production `ink_core::text_width` function at 46 px:

```sh
cargo run --release -p ink-core --example text_probe
```

| Text | Host Public Sans advance px | Android Public Sans advance px | Android default advance px |
| --- | ---: | ---: | ---: |
| Count: 0 | 151.93703 | 151.18360 | 144.96484 |
| office ffi | 156.49788 | 153.09375 | 140.07812 |
| AVATAR | 159.43405 | 143.53125 | 141.90625 |
| 1234567890 | 235.24597 | 232.79297 | 219.96094 |

These differences change alignment, wrapping and truncation near a width boundary. For example, `AVATAR` fits within 150 px through Android and exceeds that width through Ink's host path. Removing Public Sans changes the typeface as well. Gate (c) therefore fails before an atlas or runtime JNI implementation is justified.

The probe also exercises Arabic, combining marks, tabs and emoji. Their results are recorded in the evidence, but they do not establish cross-platform equivalence: host fallback availability differs from Android, and shaping may improve some scripts while changing current output. Tabular-digit feature selection, fallback identity, truncation, atlas rasterisation and every RTL case have not passed a production equivalence gate.

For an eight-string working set over 100 frames of 1,000 lookups, two frame maps reduce shaping calls from 100,000 to 8. This measures Java-to-platform shaping and map lookup. It excludes Ink's Rust-to-Java string transfer, JNI result extraction, glyph rasterisation and atlas upload. It is not a head-to-head device layout benchmark and does not pass gate (b).

Gate (a) also remains unproved for a platform-text replacement. The embedded Public Sans file is 60,816 bytes; that is an input size, not a measured removable contribution to the linked library. Code sharing and replacement JNI code require a real stripped release comparison. No platform-text candidate was built, and no native saving is claimed. The counter release library measurements below quantify the separate layout-cache cost.

To reproduce the Android API probe after compiling its Java class against `android-34/android.jar`, use `d8` to produce `classes.dex`, push that and Public Sans to `/data/local/tmp`, then run:

```sh
adb shell 'CLASSPATH=/data/local/tmp/ink-layout-text-probe.dex app_process /system/bin AndroidTextProbe /data/local/tmp/ink-layout-public-sans.ttf'
```

The probe initialises the platform font map because `app_process` does not perform application font setup. That reflection is confined to the diagnostic tool; apps do not need it.

## Benchmark measurements

Baseline source: `037c1e3b7824e66342f75f8391bf69d23bb566d8`. The final comparison reruns the saved baseline executable and freshly compiled candidate in alternating order. Both use identical compiler-produced assets and workload hashes. Timings cover host QuickJS dispatch through native scene readiness; they exclude Android launch, IME, image decoding, GPU work and presentation.

The final run uses three fresh processes per version, 20 warm-ups and 100 samples per workload per process. Negative changes mean faster. The report preserves all workloads, including regressions. Scheduling-sensitive tails are noisy; these measurements do not promise a universal speed-up.

```sh
bun run bench:core --rounds 3 --samples 100 --warmup 20 \
  --baseline .test-output/performance/2026-11-07T12-39-22.821Z-61859
```

Host: Apple M4 Pro, darwin/arm64; rustc 1.96.0 (ac68faa20 2026-05-25); Bun 1.4.4. All 30 workload contracts and idle checks passed in every round.

| Workload | Median ms, baseline → candidate | p95 ms, baseline → candidate | Median change % | p95 change % |
| --- | ---: | ---: | ---: | ---: |
| react-small | 1.3225 → 1.3135 | 1.4657 → 1.4322 | -0.7 | -2.3 |
| react-bulk | 1.4436 → 1.4312 | 1.5902 → 1.5648 | -0.9 | -1.6 |
| react-resize | 2.3140 → 2.3972 | 2.4988 → 2.5966 | 3.6 | 3.9 |
| list-edit-100 | 0.2013 → 0.1955 | 0.2938 → 0.2410 | -2.9 | -18.0 |
| list-scroll-100 | 0.0154 → 0.0139 | 0.0186 → 0.0174 | -9.8 | -6.5 |
| list-edit-1000 | 0.2521 → 0.2464 | 0.5418 → 0.4726 | -2.3 | -12.8 |
| list-scroll-1000 | 0.0154 → 0.0141 | 0.0189 → 0.0180 | -8.6 | -4.9 |
| list-edit-10000 | 0.7541 → 0.7304 | 2.3142 → 2.1522 | -3.1 | -7.0 |
| list-scroll-10000 | 0.0161 → 0.0145 | 0.0199 → 0.0181 | -9.8 | -9.2 |
| scrollbar-drag-10000 | 0.0722 → 0.0747 | 0.0797 → 0.0785 | 3.4 | -1.5 |
| rows-edit-100 | 0.2588 → 0.2270 | 0.3643 → 0.2688 | -12.3 | -26.2 |
| rows-edit-1000 | 0.3285 → 0.2990 | 0.8608 → 0.3540 | -9.0 | -58.9 |
| rows-prepend-1000 | 1.7183 → 1.6478 | 2.2273 → 1.9477 | -4.1 | -12.6 |
| rows-reorder-1000 | 1.6748 → 1.5882 | 2.2193 → 1.9574 | -5.2 | -11.8 |
| rows-native-scroll-1000 | 0.0787 → 0.0766 | 0.0882 → 0.0862 | -2.6 | -2.2 |
| rows-react-scroll-1000 | 0.9436 → 0.9085 | 1.2200 → 0.9996 | -3.7 | -18.1 |
| rows-pure-scroll-1000 | 0.1273 → 0.1103 | 0.1449 → 0.1260 | -13.3 | -13.0 |
| rows-decoded-image-arrival | 0.0028 → 0.0022 | 0.0060 → 0.0062 | -23.5 | 3.5 |
| startup-runtime-to-library | 8.5654 → 8.4947 | 9.0447 → 8.9487 | -0.8 | -1.1 |
| navigation-open | 1.0772 → 1.0746 | 1.3867 → 1.3943 | -0.2 | 0.5 |
| navigation-back | 0.2590 → 0.2632 | 0.2897 → 0.3683 | 1.6 | 27.1 |
| navigation-tab | 0.2737 → 0.2768 | 0.2837 → 0.5787 | 1.1 | 104.0 |
| conversation-controlled-input | 0.2865 → 0.2633 | 0.3010 → 0.3040 | -8.1 | 1.0 |
| conversation-append | 0.6410 → 0.6530 | 0.6978 → 0.8262 | 1.9 | 18.4 |
| conversation-prepend | 0.5125 → 0.5183 | 0.6156 → 0.6358 | 1.1 | 3.3 |
| conversation-input-background | 0.6920 → 0.6714 | 1.9851 → 3.1831 | -3.0 | 60.4 |
| conversation-input-under-load | 1.6062 → 1.6193 | 2.0423 → 2.0181 | 0.8 | -1.2 |
| playback-native-clock | 0.0001 → 0.0001 | 0.0001 → 0.0001 | 0.0 | 0.0 |
| playback-state-change | 0.1476 → 0.1491 | 0.1661 → 0.1897 | 1.0 | 14.2 |
| conversation-input-cpu | 85.5275 → 73.2207 | 165.9564 → 152.8268 | -14.4 | -7.9 |

The mean number of measured nodes falls from 132 to 7 for list edits, from 452 to 27 for row edits, and from 1,674 to 16 for controlled conversation input. React's existing fixed-width text shortcut already avoids layout in the small and bulk cell workloads. Full cell resizing still measures 1,551 nodes and adds cache overhead. Navigation tails and several conversation workloads regress; consult the absolute times and per-round samples before generalising the median gains.

Raw samples, binary hashes, execution order, source hashes and compiled assets: `.test-output/performance/2026-11-07T12-54-46.212Z-88540/`. The initial baseline bundle is retained at `.test-output/performance/2026-11-07T12-39-22.821Z-61859/`. Earlier exploratory runs exposed repeated validation overhead and are not the final candidate measurements.

## Native size and remaining limits

Counter feature set, arm64-v8a, API 34, release profile, QuickJS parser disabled, stripped `libink_android.so`:

| Build | Bytes |
| --- | ---: |
| Baseline | 2,705,264 |
| Layout reuse | 2,743,984 |
| Change | +38,720 (+1.43%) |

The ELF section listing confirms the candidate has no `.symtab` or debug sections. These measurements do not estimate a future Android-text replacement. The retained text stack gives no Part 2 size saving.

The standalone Android text probe measured 100-frame distributions for 1,000 lookups per frame:

| Probe | Median per frame ms | p95 per frame ms | Platform shaping calls |
| --- | ---: | ---: | ---: |
| Uncached | 2.121375 | 4.189042 | 100,000 |
| Two frame maps | 0.032583 | 0.045500 | 8 |

These numbers demonstrate cache call reduction for a repeated working set. They do not compare Ink's production paths or include Rust-to-Java JNI and atlas costs. Gate (c) failed; gates (a), (b) and the complete behaviour gate (d) remain unproved. The evaluation stops here, as required when any gate fails.

Final validation: 13 core tests and 18 compiled scene contracts passed. Clippy completed with 14 warnings and no diagnostics in `reuse.rs`. Saved screenshot comparisons, export comparisons, Android and host text measurements, native library hashes and emulator details are under `.test-output/reuse-evidence/`.

Remaining limits:

- Cache validation still compares native subtree content once per pass. It does not eliminate React reconciliation or every tree walk.
- Translated, rounded and clipped paint is rebuilt. No pixel translation shortcut is assumed safe.
- Native node snapshots and the previous primitive buffers increase retained memory; a heap saving has not been measured.
- Full resizing and some navigation and conversation timings regress. The benchmark table reports them alongside the gains.
- Any new reusable node kind needs an audit of measurement side effects, scene indices and inherited input behaviour.
- Emulator captures verify settled counter, list and navigation states. They do not establish physical-device GPU behaviour or every animation frame.

Final emulator comparison after restoring the probe package before input and checking foreground ownership before and after screenshots:

| State | Changed pixels / 1,339,200 | Maximum channel difference |
| --- | ---: | ---: |
| counter-0 | 0 | 0 |
| counter-1 | 0 | 0 |
| list-start | 0 | 0 |
| list-scrolled | 363 | 9 |
| detail | 0 | 0 |
| list-return | 0 | 0 |
| counter-return | 0 | 0 |

All seven states pass the existing visual tolerance. Six are pixel-identical. The scrolled state differs in 363 shaded pixels (0.0271%), with maximum channel difference 9 and identical alpha. Counter, initial list, restored list, detail and restored counter match exactly. Three production route exports also match exactly. Activity logs recorded other agents bringing the Counter app forward during discarded attempts; the final capture checks the probe package around every screenshot. The fixture uses its own application ID, `dev.ink.layoutreuse`, so installations do not replace other sessions’ apps.
