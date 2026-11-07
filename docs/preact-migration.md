---
title: "Preact runtime migration"
description: "Compatibility, validation and measurements for the Preact UI runtime."
---

Ink uses Preact 10.29.3 to reconcile application components. Apps retain React imports, TypeScript declarations, React Compiler output and fast refresh. Hidden screens disconnect app effects and external-store subscriptions while retaining component state and native nodes. The Rust bridge and native operation protocol are unchanged.

## Native reconciliation

The compiler replaces Preact's browser element driver with an Ink driver. Preact still owns component reconciliation, keyed children, context, refs and state. The driver passes original native props to Ink once, skips browser property setters and avoids allocating child arrays for primitive Text content. A SHA-256 check pins the private helper ABI to the installed Preact distribution. An upgrade fails compilation until the driver and its contracts are updated together.

Ink's JSX runtime creates the vnode fields that reconciliation needs. It omits unused browser and debug fields and skips component option hooks for native host nodes. Component vnodes retain the compatibility and Prefresh hook chain. Host nodes keep parent and sibling links and a mounted flag. Native callback references, compiled-list patches and adjacent text batches retain their existing paths. After placement, the adapter releases retired vnode links so old parent/child cycles do not retain list arrays until QuickJS cycle collection. Cached host children are reparented before the old links are released.

A QuickJS probe instrumented Preact's option hooks while applying 120 all-cell updates. The earlier renderer visited each of the Text vnode, before-diff and after-diff hooks 60,500 times, including the initial 500 cells. The native driver and JSX runtime remove those Text hook visits. This identifies redundant host work, rather than assuming that compatibility hooks on components explain the 500-cell regression. The instrumented run is diagnostic; the timing table below uses uninstrumented bundles.

Ink-owned memoised rows still use a component with `shouldComponentUpdate`. App components retain Preact's compatibility APIs, including `memo` and `forwardRef`. React Compiler output targets React 19 and uses Ink's `react/compiler-runtime` memo slots. Neither React, react-reconciler nor scheduler runs in the counter bundle.

## Hidden-screen effects

Named and default imports from `react` use Ink's `useEffect`, `useLayoutEffect`, `useSyncExternalStore` and `useImperativeHandle`. This also covers dependencies that import React hooks. Initially hidden trees do not start these effects. Hiding a visible tree cleans up layout effects before passive effects, with parents before children. Showing it starts layout effects before passive effects, with children before parents and hooks in declaration order.

Each Activity provides a stable visibility scope. Effects read this scope without subscribing every component to a visibility context. This also avoids relying on cached vnodes' parent links, which can refer to earlier trees. Nested Activities respect hidden ancestors. Activity retains the reconciled children while hidden and applies deferred parent prop changes when shown. State updates scheduled directly on retained components still run synchronously. Effects remain disconnected. External stores read a fresh snapshot when the screen reconnects.

Ink allocates genuine Preact hook slots but owns effect scheduling. Prefresh can therefore clean up the same slots and preserve or reset state using its existing signature protocol. Effect setup failures reach application error boundaries. Effect-event callbacks update during commit, including while hidden, before app layout cleanups and setups. Native visibility is applied at the first native ancestor; descendants do not acquire redundant hidden flags. Unchanged Activities skip native visibility traversal.

## Remaining differences

- Preact does not provide React's concurrent scheduling. Transitions run synchronously and deferred values update immediately. Hidden component rendering is not scheduled as background work.
- Activity suspension covers the hooks above. Class component lifecycle methods and native host refs retain Preact's lifecycle behaviour; they do not acquire React's full Offscreen lifecycle semantics. Insertion effects retain their Preact behaviour.
- APIs beyond Preact's compatibility exports and Ink's adapters are not supplied. Dependencies that rely on React 19 internals or additional React 19 APIs need individual validation. React typings preserve source compatibility and do not guarantee complete runtime equivalence.
- The native driver and effect scheduler depend on the pinned Preact ABI. Upgrade them with the renderer, navigation, visibility, compiler-cache, Suspense and refresh contracts. Imports directly from `preact/hooks` bypass Ink's React hook adapters.

## Bundle sizes

The comparison uses current-main React assets captured at `dc5cecb` and the final Preact counter bundle, compiled with the same production build settings and pinned QuickJS-ng engine. Bytecode omits function source text and retains line tables. No new APK-size claim is made: the Android release profile and parser removal changed after the original migration.

| Artifact | React bytes | Preact bytes | Change |
| --- | ---: | ---: | ---: |
| app.js | 149,087 | 39,204 | -73.70% |
| app.jsbc | 176,997 | 58,050 | -67.20% |

The additional effect lifecycle support makes this bundle slightly larger than the earlier migration's 38,059-byte JavaScript and 56,441-byte bytecode. Most of the size reduction remains; startup and update timings are measured separately below.

## Core performance

The baseline is current main at `dc5cecb`; the implementation is `a3c9b4d`. Measurements were taken from the working tree. Saved source hashes confirm that the measured runtime and contract files match the committed implementation. Both variants use the identical release benchmark executable. Measurements used Bun 1.4.4, Rust 1.96.0 and the pinned QuickJS-ng engine on an Apple M4 Pro.

`bun run bench:core --rounds 6 --samples 100 --warmup 20 --baseline RUN_DIRECTORY` provides six alternating pairs of fresh processes and 600 measured samples per workload and variant. Every process and sample is retained. Negative changes mean faster.

| Workload | React median ms | Preact median ms | Median change | React p95 ms | Preact p95 ms | p95 change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| react-small | 1.307125 | 0.922958 | -29.4% | 1.505792 | 1.107917 | -26.4% |
| react-bulk | 1.422916 | 1.040083 | -26.9% | 1.641917 | 1.208083 | -26.4% |
| react-resize | 2.361917 | 1.999083 | -15.4% | 2.696667 | 2.434208 | -9.7% |
| list-edit-100 | 0.189416 | 0.135250 | -28.6% | 0.264125 | 0.194708 | -26.3% |
| list-scroll-100 | 0.013750 | 0.013833 | 0.6% | 0.017625 | 0.018333 | 4.0% |
| list-edit-1000 | 0.243875 | 0.188417 | -22.7% | 0.422083 | 0.429375 | 1.7% |
| list-scroll-1000 | 0.013958 | 0.013917 | -0.3% | 0.017750 | 0.019084 | 7.5% |
| list-edit-10000 | 0.729750 | 0.700625 | -4.0% | 2.161250 | 1.989584 | -7.9% |
| list-scroll-10000 | 0.014750 | 0.014459 | -2.0% | 0.018167 | 0.018459 | 1.6% |
| scrollbar-drag-10000 | 0.074875 | 0.075084 | 0.3% | 0.089583 | 0.087292 | -2.6% |
| rows-edit-100 | 0.223583 | 0.168625 | -24.6% | 0.311000 | 0.231667 | -25.5% |
| rows-edit-1000 | 0.298709 | 0.240833 | -19.4% | 0.636083 | 0.513166 | -19.3% |
| rows-prepend-1000 | 1.631750 | 1.536375 | -5.8% | 2.112250 | 1.943584 | -8.0% |
| rows-reorder-1000 | 1.581125 | 1.477792 | -6.5% | 1.997292 | 1.829375 | -8.4% |
| rows-native-scroll-1000 | 0.076500 | 0.076250 | -0.3% | 0.092417 | 0.086625 | -6.3% |
| rows-react-scroll-1000 | 0.886292 | 0.794000 | -10.4% | 1.100709 | 0.891542 | -19.0% |
| rows-pure-scroll-1000 | 0.110250 | 0.109750 | -0.5% | 0.133292 | 0.125833 | -5.6% |
| rows-decoded-image-arrival | 0.002583 | 0.002458 | -4.8% | 0.007167 | 0.005916 | -17.5% |
| startup-runtime-to-library | 8.526125 | 3.524834 | -58.7% | 9.367708 | 3.956917 | -57.8% |
| navigation-open | 1.094875 | 1.067917 | -2.5% | 1.451417 | 1.380083 | -4.9% |
| navigation-back | 0.264166 | 0.285500 | 8.1% | 0.388041 | 0.317916 | -18.1% |
| navigation-tab | 0.274500 | 0.207666 | -24.3% | 0.472625 | 0.259291 | -45.1% |
| conversation-controlled-input | 0.255542 | 0.197166 | -22.8% | 0.328958 | 0.249792 | -24.1% |
| conversation-append | 0.640916 | 0.579416 | -9.6% | 0.938208 | 0.776167 | -17.3% |
| conversation-prepend | 0.514334 | 0.445292 | -13.4% | 0.781708 | 0.508083 | -35.0% |
| conversation-input-background | 0.799042 | 0.696208 | -12.9% | 2.478083 | 2.333250 | -5.8% |
| conversation-input-under-load | 1.778875 | 1.703916 | -4.2% | 2.110625 | 2.280500 | 8.0% |
| playback-native-clock | 0.000125 | 0.000125 | 0.0% | 0.000125 | 0.000125 | 0.0% |
| playback-state-change | 0.151209 | 0.097500 | -35.5% | 0.197334 | 0.186583 | -5.4% |
| conversation-input-cpu | 77.372708 | 74.692333 | -3.5% | 157.390791 | 158.443833 | 0.7% |

### Noise bound

A React-versus-React capture ran the same saved React executable and assets in ten alternating pairs: 20 fresh processes in total, each with 20 warm-ups and 100 samples per workload. For each workload and metric, the absolute tolerance is the largest per-process value minus the smallest per-process value across those controls. The tolerances were frozen before the final candidate comparison. The comparison passes when the candidate’s pooled metric is no greater than the baseline’s pooled metric plus that tolerance. All 30 workloads pass for both median and p95.

This is an observed range, not a confidence interval. It is workload-specific rather than a single percentage. The tolerances include process scheduling and collection variation. They are not a claim that every raw timing is lower: navigation-back is +8.1% on median (0.021334 ms), within its 0.028250 ms React-only control range, and faster at p95. Native pure-row scrolling is within noise on median and faster at p95. All per-workload tolerances are listed below.

| Workload | Median tolerance ms | p95 tolerance ms |
| --- | ---: | ---: |
| react-small | 0.113208 | 0.607542 |
| react-bulk | 0.135459 | 0.773750 |
| react-resize | 0.342957 | 1.139584 |
| list-edit-100 | 0.045333 | 0.174333 |
| list-scroll-100 | 0.001750 | 0.006041 |
| list-edit-1000 | 0.050792 | 0.300207 |
| list-scroll-1000 | 0.002125 | 0.011583 |
| list-edit-10000 | 0.168083 | 0.690917 |
| list-scroll-10000 | 0.001458 | 0.013793 |
| scrollbar-drag-10000 | 0.006583 | 0.045666 |
| rows-edit-100 | 0.041083 | 0.270792 |
| rows-edit-1000 | 0.066583 | 0.373167 |
| rows-prepend-1000 | 0.328000 | 0.800458 |
| rows-reorder-1000 | 0.308542 | 0.863125 |
| rows-native-scroll-1000 | 0.008125 | 0.030458 |
| rows-react-scroll-1000 | 0.103126 | 0.356084 |
| rows-pure-scroll-1000 | 0.009958 | 0.124542 |
| rows-decoded-image-arrival | 0.002042 | 0.005834 |
| startup-runtime-to-library | 0.826209 | 1.108375 |
| navigation-open | 0.068542 | 0.470792 |
| navigation-back | 0.028250 | 0.136792 |
| navigation-tab | 0.018041 | 0.417500 |
| conversation-controlled-input | 0.047959 | 0.134709 |
| conversation-append | 0.087417 | 0.566792 |
| conversation-prepend | 0.078000 | 0.294208 |
| conversation-input-background | 0.309499 | 1.326541 |
| conversation-input-under-load | 0.408792 | 0.672292 |
| playback-native-clock | 0.000042 | 0.000125 |
| playback-state-change | 0.038126 | 0.373417 |
| conversation-input-cpu | 27.709208 | 27.310708 |

The 500-cell fixture measures dispatch through native scene application. The benchmark excludes Android process launch, IME, GPU submission and display presentation. Startup creates a fresh engine with JavaScript source already read; it does not measure Android bytecode launch. Image arrival starts with decoded RGBA. Playback clock updates run natively. Sub-microsecond timings need an absolute noise comparison; rounded percentage changes alone are not application-level evidence.

## Validation

`bun run test` passed 177 checks: unit 18, compiler 52, SDK 52, runtime 29, scene 18 and audio 8. This includes actual QuickJS runtime and native navigation/list paths. `bun run check`, counter and light-template TypeScript checks, and `bun run docs:validate` passed.

Nine bundler and renderer contracts cover current callbacks, primitive and mixed text batches, retained hidden-screen state, subscription reconnection, app and library effect order, initially hidden and nested Activities, imperative handles, error boundaries, refresh retention/reset and Suspense fallback replacement. Effects from a render that suspends are not started. Cached host children retain context updates and error-boundary ancestry. Effect-order expectations were checked against current main's actual React renderer, then recorded as explicit expected sequences. They do not compare one implementation with another at test time.

The Light Phone III emulator was launched with `emulator -avd Light_Phone_III -writable-system`. The native Android host was rebuilt once after rebasing. React and Preact APK pairs contain identical native libraries and their own freshly compiled bytecode. Counter frames at counts 0 and 1 match React across all 1080 × 1240 pixels. The tap changes 14,723 pixels. A separate, temporary AVD directory isolated the comparison from another session using the existing emulator. The full navigation app’s 5,000-row list differs at one pixel initially and 234 pixels at the bottom, with maximum channel differences of 1/255 and 5/255 respectively. These differences occur at antialiased glyph and arrow edges. Repeating each variant reproduces its saved pixels exactly; the full navigation frames are therefore not bit-identical. Saved pixels, rather than previews, establish these comparisons. The temporary emulator was stopped and its disk image removed.

A production-compiled Activity fixture ran on the emulator. Its 20 layout, passive and subscription events matched the React-derived expected sequence, with no subscriptions remaining after removal. The fixture changed props and published store data while hidden before showing and removing the tree.

Local evidence, relative to this worktree:

- Current-main React executable, assets and source hashes: `.test-output/performance/2026-11-07T14-09-46.827Z-37374`.
- React-versus-React raw rounds and noise calculations: `.test-output/preact-performance/react-noise-current` and `.test-output/preact-performance/noise-current-bounds.json`.
- Final comparison: `.test-output/preact-performance/final-current-comparison` (aggregate), `.test-output/performance/2026-11-07T14-13-34.414Z-39364` (all raw rounds).
- Full test logs and source hashes: `.test-output/2026-11-07T14-15-41.183Z-40352`.
- Final emulator APKs, PNGs, pixel counts, Activity fixture and logs: `.test-output/preact-performance/emulator-current`. Earlier snapshot captures remain in `.test-output/preact-performance/emulator`.
- QuickJS profiling sources and observations: `.test-output/preact-performance/profile` and `.test-output/preact-performance/host-probes`.
