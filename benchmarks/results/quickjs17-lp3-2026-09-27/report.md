# QuickJS-ng 0.17.0 upgrade and cleanup

27 September 2026. Updated to [QuickJS-ng 0.17.0](https://github.com/quickjs-ng/quickjs/releases/tag/v0.17.0), the latest release checked today.

`rquickjs` and `rquickjs-core` remain on published 0.14.0. The workspace patches only `rquickjs-sys` to upstream commit `7a23f6f02f5396b41283b25a0d72f34d8982778e`. Its engine submodule is the exact 0.17.0 release, `6d46d07d04041b40f4f49eaa7fdebe44c314c699`. Cargo.lock pins the dependency; no vendored engine, custom fork or build-time “latest” lookup was introduced. Remove the patch once a published binding release includes this engine.

## Cleanup

Reviewed the accumulated Rust, TypeScript, Kotlin, build and probe changes. Removed a duplicate hit-region lookup during pointer-up and redundant GlyphId qualification. Kept the measured fast paths, narrow FFI safety explanation, memory limits, native scrollbar behaviour, silent action handling and compatibility checks. Avoided unrelated formatting/refactoring. No permanent tests were added.

## Physical LP3 comparison

Same 500 mounted text cells with resize enabled, no virtualisation. ABBA order: baseline-a, candidate-a, candidate-b, baseline-b. One discarded warm-up and two measured three-second probes per installation; 60 accepted taps each, zero rejected. Both use bridge timing instrumentation. The candidate includes the small cleanup above.

| Metric | 0.16.2 | 0.17.0 |
| --- | ---: | ---: |
| Input → submission median | 14.91 ms | 15.74 ms |
| p95 | 34.13 ms | 32.27 ms |
| Samples below 16 ms | 34/60 | 34/60 |
| APK bytes | 2,742,720 | 2,743,120 |

The new median is 0.82 ms higher in this comparison; p95 is lower. This does **not demonstrate a speed gain**. CPU clocks were not fixed and runs vary, so this short comparison does not isolate a small engine-only regression either. The release is retained to fulfil the requested upstream upgrade, with its correctness fixes. The APK increase is 400 bytes.

Old build: `experiment-20260927-211635-c9af95d9`. New: `experiment-20260927-214134-99911fea`. See [summary](summary.json), [run order](runs.json), build manifests and individual probe directories.

Three continuous-update probes on `experiment-20260927-214254-ef748fb9` produced 180 commits each and 179, 179 and 180 separately matched submissions, at 60.09, 60.02 and 60.23 submissions/second respectively. The first was warm-up. One commit had no separately matched submission in each of the first two runs. These probes measure throughput, not request-to-submission latency or physical screen cadence; they are not a new paired sustained-latency comparison.

## Validation

- `cargo check --locked -p ink-runtime -p ink-core -p ink-renderer-vulkan -p ink-compiler`.
- `bun run check`, and template/paced TypeScript checks.
- Android release builds of update benchmark, paced benchmark and normal template.
- Existing actual AppRuntime compatibility fixture output exactly matched the previous version (commits, callbacks, refs, links, errors, unmounts).
- Existing runtime harness passed UTF-8 streaming/invalid inputs, Japanese/emoji, detached/resized buffers, timers, microtasks, cancellation, allocation/cycle stress and the 64 MiB memory limit.
- Upstream 0.17.0 language, closure and loop checks passed, plus existing property/getter/proxy/GC checks and 16,384 ASCII segmentation comparisons.
- Normal template smoke-tested in the visible emulator: navigation, scrolling, Japanese, combining marks, mixed scripts and emoji. Screenshots included. Template build: `experiment-20260927-214348-27606198`.
- `git diff --check` passed. Template package configuration restored after building an isolated test package.

Temporary harnesses remain outside production source. These are targeted compatibility checks, not exhaustive testing of every app feature. Earlier benchmark reports retain their original engine versions and results.
