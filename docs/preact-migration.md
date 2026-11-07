---
title: "Preact runtime migration"
description: "Compatibility, validation and measurements for the Preact UI runtime."
---

Ink now uses Preact 10.29.3 to reconcile application components. Apps retain React imports, JSX and TypeScript declarations. The native operation protocol and Rust implementation are unchanged.

## Implementation

The host facade stores parent and sibling links, so inserting, moving and removing a sibling takes constant time. It retains native prop filtering, current callback references, compiled-list patches and adjacent text batches. Primitive Text content remains on the host node; mixed text creates string-valued child nodes.

Ink components import Preact core through a typed adapter. Memoised Ink rows use a class with `shouldComponentUpdate`. App React imports use Preact compatibility exports, with Ink adapters for `Activity` and `useEffectEvent`. JSX uses Preact’s JSX runtimes. React Compiler output still targets React 19 and receives memo slots from Ink’s `react/compiler-runtime` alias. Neither React, react-reconciler nor scheduler appears in the counter bundle’s resolved runtime inputs.

Prefresh replaces React Refresh. The adapter translates Oxc’s component registrations and hook signatures. Development root rendering waits until replacement finishes, preserving state for body edits. Hook-signature changes reset state. Existing module caching and native rebuild checks remain active.

## Behaviour differences and risks

- Hidden screens retain component state and their native nodes. Ink hooks disconnect their effects, resource subscriptions and native-module subscriptions while hidden, then reconnect when shown. App effects and external-store subscriptions imported from `react` stay active while hidden. This differs from React Activity’s effect suspension.
- Preact does not provide React’s concurrent scheduling. Transitions run synchronously and deferred values update immediately.
- React APIs beyond Preact’s compatibility exports and Ink’s two adapters are not supplied. Third-party dependencies that rely on React 19 internals or additional React 19 APIs need individual validation. React typings remain for source compatibility and do not guarantee complete React runtime equivalence.
- The renderer uses the option-hook ABI of the pinned Preact version. Upgrade Preact with the renderer, navigation, visibility and refresh checks. Native host vnodes bypass compat’s DOM attribute and event-name normalisation; component vnodes retain its hook chain.
- Custom-component reconciliation is slower in the measured workload below. The migration does not establish performance parity with the earlier minimal Preact prototype. Custom-row scrolling and navigation opening also regressed. These remain open performance risks.

## Release sizes

Both counter APKs were built from `examples/counter` with `scripts/ink build`, `ANDROID_HOME=$HOME/Library/Android/sdk`, and the same temporary signing key configured in `ink.toml`. The signing configuration was restored and the key was not committed. The bundled native library has the same SHA-256 in both APKs: `00d5b1463256d6e63764f9272f447412f65941c8874e2067378f19d99bf168b7`.

| Artifact | React bytes | Preact bytes | Change |
| --- | ---: | ---: | ---: |
| app.js | 149,087 | 38,059 | -74.47% |
| app.jsbc | 176,997 | 56,441 | -68.11% |
| counter.apk | 3,104,478 | 3,041,754 | -2.02% |

The baseline is revision `ac12a97fb369c024fc2b4de9269f137c5d70271b`. Measurements used Bun 1.4.4, Rust 1.96.0 and the pinned QuickJS-ng engine on an Apple M4 Pro. Both builds used the same toolchain. These are working-tree measurements; saved source hashes and bundle hashes identify the inputs.

## Core performance

The comparison uses `bun run bench:core --rounds 3 --samples 100 --warmup 20 --baseline RUN_DIRECTORY`. React and Preact ran in alternating order with 30 workloads in each fresh process. All work contracts and idle observations passed. Medians combine 300 samples per workload. Negative changes mean faster; p95 changes describe the tail separately.

| Workload | React median ms | Preact median ms | Median change | p95 change |
| --- | ---: | ---: | ---: | ---: |
| react-small | 1.314 | 2.417 | 84.0% | 86.7% |
| react-bulk | 1.451 | 2.688 | 85.2% | 90.4% |
| react-resize | 2.288 | 3.703 | 61.8% | 65.0% |
| list-edit-100 | 0.196 | 0.139 | -29.1% | -4.8% |
| list-scroll-100 | 0.015 | 0.015 | 0.3% | 0.5% |
| list-edit-1000 | 0.244 | 0.194 | -20.8% | 29.6% |
| list-scroll-1000 | 0.015 | 0.015 | -3.0% | -0.7% |
| list-edit-10000 | 0.748 | 0.688 | -8.1% | -6.9% |
| list-scroll-10000 | 0.016 | 0.016 | -0.5% | -0.9% |
| scrollbar-drag-10000 | 0.070 | 0.070 | 1.1% | 1.0% |
| rows-edit-100 | 0.236 | 0.182 | -23.1% | -54.5% |
| rows-edit-1000 | 0.309 | 0.255 | -17.7% | 3.0% |
| rows-prepend-1000 | 1.578 | 1.497 | -5.2% | -9.3% |
| rows-reorder-1000 | 1.543 | 1.458 | -5.5% | -4.3% |
| rows-native-scroll-1000 | 0.079 | 0.078 | -0.9% | 1.5% |
| rows-react-scroll-1000 | 0.896 | 1.040 | 16.0% | 17.0% |
| rows-pure-scroll-1000 | 0.126 | 0.128 | 2.0% | 8.8% |
| rows-decoded-image-arrival | 0.002 | 0.003 | 38.0% | 27.6% |
| startup-runtime-to-library | 8.850 | 4.107 | -53.6% | -53.8% |
| navigation-open | 1.082 | 1.318 | 21.8% | 14.3% |
| navigation-back | 0.265 | 0.291 | 9.6% | 1.1% |
| navigation-tab | 0.273 | 0.225 | -17.6% | 35.7% |
| conversation-controlled-input | 0.277 | 0.235 | -15.0% | -12.3% |
| conversation-append | 0.640 | 0.601 | -6.2% | 12.1% |
| conversation-prepend | 0.510 | 0.455 | -11.0% | -1.1% |
| conversation-input-background | 0.588 | 0.604 | 2.7% | 16.3% |
| conversation-input-under-load | 1.730 | 1.572 | -9.1% | 2.1% |
| playback-native-clock | 0.000 | 0.000 | 48.8% | 0.0% |
| playback-state-change | 0.147 | 0.095 | -35.5% | -19.5% |
| conversation-input-cpu | 74.815 | 73.562 | -1.7% | 0.5% |

The 500-cell fixture measures dispatch through native scene application. These timings exclude Android launch, IME, GPU submission and display presentation. Values below 0.001 ms round to zero; percentage changes in those cases are not useful evidence of an application-level difference.

## Validation

`bun run test` passed 158 checks: unit 9, compiler 42, sdk 52, runtime 29, scene 18, audio 8. This includes workspace Rust tests and the real QuickJS runtime, scene, navigation, data and audio suites.

Four new integration checks execute the real production or development bundler and renderer. They cover current callbacks and primitive/mixed text batches, hidden-screen state and subscription reconnection, refresh retention/reset, and Suspense fallback replacement. They use explicit expected operations and messages.

`bun run check`, counter and light-template TypeScript checks, and `bun run docs:validate` passed. The broader `ink check` command for light-template stops at existing formatting issues in `app/modules/overlays.tsx` and `overlays.tsx`; those files were left unchanged.

The Light Phone III emulator was launched with `emulator -avd Light_Phone_III -writable-system`. Release counter screenshots at counts 0 and 1 match the React baseline pixel-for-pixel across all 1080 × 1240 pixels. The counter responds to taps. An on-device Prefresh body edit retained count 1; adding a hook reset the count to 0, verified against the corresponding baseline count regions. Temporary source edits were restored.

The light-template’s 5,000-row virtualised-list page rendered, switched from Start updates to Stop updates, reached Updates: 28, and scrolled from rows 1–3 to rows 3–7. Saved image comparison found 35,337 changed pixels in the list viewport. OCR read the title, update state and row labels from the saved files.

Local evidence directories, relative to the worktree:

- Sizes, APKs and emulator screenshots: `.test-output/preact-migration`.
- Baseline executable and assets: `.test-output/performance/2026-11-07T12-15-36.065Z-16117`.
- Alternating benchmark, raw samples and source hashes: `.test-output/performance/2026-11-07T12-31-27.973Z-53935`.
- Full test logs and source hashes: `.test-output/2026-11-07T12-31-28.505Z-53959`.
