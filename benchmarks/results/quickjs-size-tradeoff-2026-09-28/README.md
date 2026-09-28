# QuickJS size versus speed after the native UI refactor

28 September 2026. This is an investigation, not a change to the production optimisation defaults. They remain `-O3` for QuickJS and Rust. No apps were installed for this experiment.

## Recommendation

Keep QuickJS at `-O3` for Ink's current speed-first goal. `-Os` is a viable explicit size trade-off, but the Rust refactor does not make its performance cost negligible. `-Oz` is a much worse trade-off in these workloads.

Rust owns layout, drawing, native scrolling and the standard native displays. JavaScript still runs initial screen construction, app callbacks, collection projection, React compatibility, value preparation and parts of transport. "Most UI machinery is Rust" does not imply that most update time is independent of QuickJS. Even the native bindings fixture has a measurable interpreter cost.

## Android native library size

Same archived Counter source, NDK 29, arm64, Rust `opt-level=3`, fat LTO, one codegen unit, panic abort. Only the QuickJS C optimisation flag changes. `cargo tree -p ink-android --target aarch64-linux-android -i cc` confirms QuickJS is the only C build using `cc` in this configuration.

| QuickJS | Stripped native library | Saving against diagnostic O3 |
| --- | ---: | ---: |
| O3 | 2,517,664 B | — |
| Os | 2,223,200 B | 294,464 B |
| Oz | 2,122,264 B | 395,400 B |

These are standalone shared-library builds, not newly packaged/signed APKs. The symbol-bearing diagnostic build, stripped afterwards, is 640 bytes larger than the shipped Counter native library. Compare diagnostic builds with each other. Applying their deltas to the 2,692,824-byte APK suggests approximately **2.40 MB with Os** or **2.30 MB with Oz**, subject to APK alignment/signing effects. Those APK sizes are estimates.

The size flags change machine-code optimisation, not the supported JS language features. Clang describes [Os and Oz](https://clang.llvm.org/docs/CommandGuide/clang.html#code-generation-options); actual performance depends on the workload and target compiler.

## Repeated Mac CPU measurements

Six balanced-order rounds per fixture and mode; 200 measured updates after 20 warm-ups. Seventy-two prepared harness executions in total, using the same assets across modes. Builds finished before this comparison. Existing scene/callback/Unicode/list regression checks ran in every applicable harness invocation and passed.

All builds retain the existing macOS `-fno-unroll-loops`; Os/Oz are appended only for QuickJS. Rust stays O3. Timings include real JS/Rust queue waits and scene layout, but exclude Android scheduling, Vulkan, physical input and display. These Mac results do not predict exact LP3 slowdowns.

| Mean CPU duration (ms) | O3 | Os | Oz |
| --- | ---: | ---: | ---: |
| Native bindings: 500 cells + resize | 0.273 | 0.303 | 0.457 |
| React: 500 cells + resize | 1.686 | 2.025 | 3.557 |
| Public list: one-row edit in 1,000 records | 1.249 | 1.474 | 2.536 |
| Native controls: replace 1,000 records | 2.264 | 2.576 | 4.003 |
| Native-only scrolling, native controls | 0.0594 | 0.0591 | 0.0580 |
| Native bindings mount/setup | 13.30 | 14.42 | 21.65 |

Os costs approximately 11% on native bindings, 20% on the React update, 18% on the one-row list edit and 14% on bulk native-control data replacement. Oz costs approximately 67%, 111%, 103% and 77%, respectively. Small native-scroll differences are noise; this path does not run JavaScript. All scenes remaining correct is not a complete application compatibility guarantee.

Raw round results: `paired-cpu.json`. Aggregates: `summary.json`. Harness IDs, build environments, source hashes and binary hashes: `artifacts.json`.

## Follow-up candidates at the time of measurement

The entry-point and platform-polling work below has since been implemented and validated: see [Android boundary cleanup](../android-boundary-2026-09-29/README.md) and [portal transport](../portal-cache-2026-09-29/README.md). The prototype measurements remain here for provenance.

### Finish capability gating at Android entry points

The previous work gated component construction and layout. Exported JNI entry points can still keep otherwise unused code linked. Counter retains editor-context serialisation, text editing, input notifications, camera-review checks and camera/map/video portal serialisation.

An isolated source experiment gates nine relevant JNI entry points with their existing native features and gates input notifications while preserving viewport notifications. This last distinction matters: the same method delivers list viewport events, which must continue without text-input support.

| O3 diagnostic library | Bytes |
| --- | ---: |
| Current | 2,517,664 |
| Entry-point gating experiment | 2,488,424 |
| **Saving** | **29,240** |

This is a measured code-size opportunity, **not an implemented or device-validated production change**. `entry-gates.patch` records the exact prototype. Finishing it should include the matching Kotlin call sites and checks for absent capabilities, keyboard input, dynamic native views, camera, maps and video. Counter should not call adapters for features it cannot use. No speed gain has been measured for this prototype.

### Avoid unnecessary platform polling and serialisation

`MainActivity.syncTextInput` makes four native calls, including context serialisation. `syncCameraPortal` asks for camera, camera-review, video and map state. These helpers run from scene/update paths even when those capabilities are absent. Capability-aware calls and publishing changes only when relevant state changes could reduce crossing/allocation overhead without slowing QuickJS. Maintain focus, cursor, keyboard and portal geometry correctness. Additional savings beyond the prototype are unmeasured.

### Broaden component selection carefully

The generic core measurement/layout dispatch still supports unused controls, image interaction and media-related node kinds. Narrowing their reachability could save more code, but shared glyph/image rendering must remain for Unicode and emoji. No extra savings estimate is claimed. Approximately 797 KB of current native machine code is QuickJS C/helpers; changing small UI components alone cannot remove that floor. Symbol attribution is approximate because LTO inlines across modules; `native-profile.json` records groups and the largest symbols.

The Rust standard-library/diagnostic footprint remains substantial, but removing backtraces or rebuilding the standard library carries a separate diagnostics/toolchain cost. It is not the next recommendation here.

## Cleanup and reproducibility

The ordinary O3 host harness and Android release library were rebuilt after the experiments. Production source and optimisation settings were not changed by the prototypes. The archived source comes from `experiment-20260928-233457-5e93e73a`; scripts, the isolated patch, library sizes, sections and hashes are retained. The extracted 70 MB source copy was removed after analysis. Experimental shared libraries remain local evidence, not installed applications.
