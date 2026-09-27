# Native engine size and performance — 27 September 2026

> Historical experiment: the size-focused profile, experimental standard-library rebuild and `rust-src` requirement were subsequently reverted at the user’s request. The measurements below describe the preserved candidate, not the current source configuration. Independent build fixes, benchmark tooling and the Japanese page remain.

The counter APK is **1,974,652 bytes**, below the 2 MB target. Its native engine is **1,795,808 bytes**, down **717,112 bytes (28.5%)**. The APK retains an uncompressed, directly mapped native library. Nothing was moved into another app, downloaded separately or removed from JavaScript's supported features.

These are ARM64 release builds signed with a development key, without benchmark instrumentation. MB means 1,000,000 bytes; memory measurements use MiB. The result does not meet the 1 MB stretch target or establish a globally smallest or fastest engine.

## Counter profile

| Item | Before (bytes) | After (bytes) | Reduction |
| --- | ---: | ---: | ---: |
| APK | 2,691,764 | 1,974,652 | 717,112 |
| Native engine | 2,512,920 | 1,795,808 | 717,112 |
| Machine code (`.text`) | 1,876,960 | 1,237,648 | 639,312 |
| Read-only data (`.rodata`) | 333,332 | 303,216 | 30,116 |
| Unwind records (`.eh_frame`) | 183,464 | 151,012 | 32,452 |
| Unwind index (`.eh_frame_hdr`) | 26,364 | 25,644 | 720 |
| Relocatable read-only data | 63,912 | 58,856 | 5,056 |
| Exception tables | 4,484 | 0 | 4,484 |

Sections above are selected contributors, not an exhaustive sum. [The machine-readable profile](profile.json) contains all sections, hashes and the largest functions. The packaged and symbol-bearing binaries have byte-identical `.text` sections, checked before attributing functions.

| Approximate machine-code ownership | Before (bytes) | After (bytes) |
| --- | ---: | ---: |
| QuickJS C and compiler helpers | 824,460 | 526,740 |
| Ink | 367,352 | 417,740 |
| Rust standard library | 428,036 | 106,136 |
| JNI bindings | 86,484 | 53,928 |
| Fonts and Unicode | 50,572 | 53,264 |
| JSON / Serde | 63,508 | 51,844 |
| Other Rust | 35,880 | 18,144 |
| QuickJS Rust bindings | 20,260 | 9,788 |

These are symbol groups, not independent crate sizes. Link-time optimisation moves inlined code between groups; Ink's larger attributed total does not imply a larger engine. Padding is excluded, and the QuickJS C group includes compiler helpers.

## What changed

- Android releases use `ink-release`: size optimisation for general code and dependencies, with optimisation level 3 for `ink-core` and `ink-renderer-vulkan`. The host CLI and debug builds retain their existing profiles.
- The pinned Rust 1.96.0 toolchain rebuilds the standard library without Rust backtrace generation and symbolisation. Panic messages and native unwind tables remain. This requires `rust-src` and **experimental Cargo `build-std`**, enabled with `RUSTC_BOOTSTRAP=1` only for the native release command. Toolchain upgrades need revalidation.
- Gradle supplies its configured NDK and the app's existing minimum Android API level (34) to Cargo. The NDK compiler runtime supplies ARM atomic helpers. Undefined native symbols now fail the link instead of failing on device.
- Generated permission manifests now include the application ID in their Gradle cache inputs. This fixes stale provider authorities when switching app identities.
- The template has a Japanese text page immediately after Emoji.

An initial standard-library rebuild lacked ARM atomic helpers and failed to load on the emulator. That candidate was rejected. The final build uses `compiler-builtins-c` with the NDK compiler runtime and passes strict linking, installation and execution.

## Performance

Three alternating baseline/candidate rounds used the image-heavy 500-row scroll fixture on the visible Light Phone III emulator. Both were instrumented builds using Rust 1.96.0 and NDK 29.0.14206865. Foreground and thermal checks passed; samples reported no dropped frames or runtime errors.

| Median measurement | Before | After |
| --- | ---: | ---: |
| CPU time during interaction | 470 ms | 480 ms |
| Idle proportional set size | 25.72 MiB | 25.02 MiB |
| PSS after interaction | 27.24 MiB | 26.55 MiB |
| Continuous-scroll p99 frame time | 24 ms | 27 ms |

Memory use improved. CPU time was 2.1% higher and the frame-time tail was worse in this sample. This is **not a demonstrated speed improvement**, and three emulator rounds cannot rule out a performance regression or predict physical-device behaviour. Layout and rendering remain speed-optimised to limit the trade-off. [Raw paired measurements and build manifests](paired-benchmark.json) preserve the samples and source hashes.

An earlier 5,000-row template run measured layout p95 at 0.101 ms, preparation at 0.042 ms and frame time at 1.642 ms over 301 frames. An exploratory CPU profile had 739 samples, only 153 inside Ink's native library; 105 of those were in `JS_CallInternal`. That small sample suggests JavaScript execution is a more useful CPU target than shaving JNI wrappers, but does not establish a universal bottleneck.

## Fast iteration

Build an instrumented fixture once, install it and reserve the device. With the fixture already foregrounded at the same scroll position and cache state:

```sh
scripts/agent-tools probe --serial emulator-5554 --token "$DEVICE_TOKEN" \
  --package com.vandam.benchmark.ink.rendererscroll
```

The measurement takes about three seconds (3.06 seconds in validation), excluding setup and evidence collection. It records CPU time, renderer timing, uploads/cache misses, process continuity and a screenshot. The counter variant uses `--scenario counter` and performs 15 taps in approximately three seconds. It does not install, relaunch or clear the app. Short probes guide iteration; paired runs and compatibility checks remain necessary before accepting changes.

## Template APK

The final non-instrumented compatibility fixture is **32,532,651 bytes**. Its Ink engine is **1,850,048 bytes (5.7%)**. It includes more native capabilities than the counter. The fixture uses a separate application ID and the emulator LightOS service; existing template and counter installations were preserved.

| Largest contributor | Bytes stored in APK |
| --- | ---: |
| MapLibre native library | 11,591,520 |
| Demo video | 8,579,335 |
| Demo audio | 4,043,295 |
| Android bytecode (`classes.dex`) | 3,344,408 |
| Ink native engine | 1,850,048 |
| Three demo images | 1,913,023 |
| Android resources | 334,556 |

[Complete APK entry profile](template-profile.json). Demo assets and MapLibre were retained; their size is not attributed to Ink's core engine.

## Compatibility evidence

The final production template visited all 19 example destinations. Dynamic UI added and cleared items, and the 5,000-row list updated and scrolled. Links rendered its inline link and loaded its network preview after a longer wait. Typography, icons, image display, video controls, follow-new-items, conversation, playing screen, rows, reordering, pagination, code generation, confirmation and screen-state pages received smoke visits. No runtime errors were found in the captured final log.

Japanese checks covered hiragana, katakana, kanji, composed/decomposed dakuten, mixed Latin text and emoji. Japanese top, emoji top/lower, typography and the Wallsocket image matched the baseline pixel-for-pixel. Japanese lower screenshots used different scroll offsets and were inspected visually rather than claimed as identical.

- [Japanese page](final-japanese-text.png)
- [Kanji, combining marks and mixed text](final-japanese-text-lower.png)
- [Emoji page](final-emoji.png)
- [Loaded Links page](final-links-verified.png)

The exact profiled counter native library was also installed in a separately signed preview APK under the `.profile` application ID to preserve the existing app. It launched and advanced from Count: 0 to Count: 15 after 15 taps. [Counter screenshot](final-counter-after.png). The distributed counter APK remains the original non-instrumented release build measured above.

These checks do not establish exhaustive component behaviour. Text input was visited, not typed into; video controls were visible, but playback was not fully exercised. Physical-device hardware integrations were not tested in this run.

## Rejected variants

The exploratory matrix used a different NDK configuration, so its absolute sizes should not be compared directly with the final controlled profile.

- Compressing the APK's native entry reduced download size but left the engine unchanged. Rejected; native libraries remain uncompressed.
- Disabling unwind tables for the final crate made the binary slightly larger. Rejected.
- Optimisation level `z` with the hot crates at level 3 was 11,208 bytes larger than `s`. Rejected.
- C thin LTO saved only 2,096 bytes in its comparison, adding build complexity. Not retained.
- Applying size optimisation to the hot Ink crates was smaller, but the selected configuration spends roughly 55 KB to retain speed optimisation for layout and rendering.

## Next investigations

| Priority | Area | Evidence and next experiment |
| --- | --- | --- |
| 1 | JavaScript-to-native updates | Profile repeated state changes and operation deserialisation. Reduce redundant work or encoding overhead while retaining semantics. QuickJS dominates the short native CPU sample; fewer calls and allocations may improve both code and runtime costs. |
| 2 | Layout and React tree code | `layout_node_inner` is 20,340 bytes; `render_node` is 19,656. Look for duplicated cold paths and repeated measurements. Keep hot loops fast; use row probes and paired validation. |
| 3 | Read-only data | 303,216 bytes remain. Attribute tables and embedded strings before changing them; retain Unicode and font coverage. |
| 4 | JNI and JSON specialisation | Roughly 106 KB across those symbol groups. Identify duplicate generic instantiations or conversions before replacing interfaces. The sample does not show them as primary CPU bottlenecks. |
| 5 | Native diagnostics | Unwind records/index remain about 177 KB. Keep them for native crash diagnosis; removing diagnostics is a feature trade-off, not free efficiency. |

Reaching 1 MB would require substantial further architectural work. The current evidence supports a smaller, working engine with lower observed memory use, not a claim that every further optimisation has been exhausted.

## Reproduction evidence

- Baseline revision: `924c1d5c1d8847f2b180f9c9e7b8e6c34319a05b`.
- Baseline experiment: `experiment-20260927-131519-b7c1890d`.
- Candidate experiment: `experiment-20260927-131722-bf254c44`.
- Paired benchmark: `bench-20260927-131843-5e7bd5b5`.
- Production profiles and symbol dumps: `.agent-tools/native-next-20260927/production-before/` and `production-after/`.
- Compatibility logs: `.agent-tools/native-next-20260927/compatibility-final.log` and `final-errors.txt` (empty).
- CLI compile check, template type check, Python compile check and `git diff --check` passed. Rust test harnesses contained zero tests and are not counted as behavioural coverage.
