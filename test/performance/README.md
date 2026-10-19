# Headless performance

Run from the repository root:

```sh
bun run bench
bun run bench --rounds 1 --samples 20 --warmup 4
bun run bench --baseline .test-output/performance/<previous-run>
```

This is opt-in and separate from `bun run test`. It builds release Rust and production-compiled JavaScript, then executes the real QuickJS runtime, commit transport, React tree and Rust layout engine. No Bun execution substitutes for QuickJS, and no emulator is required.

## Workloads

| Workload | Independent observations and work contracts |
| --- | --- |
| `react-small` | Change one cell among 500 persistent controls. Verify every authored cell, unchanged neighbouring content and stable host-node count, with no layout measurement or full scene rebuild. |
| `react-bulk` | Change all 500 cells. Verify all 500 strings after every operation, in order, with no layout measurement or full scene rebuild. |
| `react-resize` | Change all 500 labels and their widths together. Check strings, proportional width changes and that cells remain inside the vertical clip. |
| `list-edit-100/1000/10000` | Edit the first visible record in an immutable array. Verify the changed and neighbouring labels, stable mounted-node count, exactly one projected row, and no transfer of the dataset or key order. The JS scan is still O(n); its time is included. |
| `list-scroll-100/1000/10000` | Scroll forward and back through native windows. Check movement, valid row content, ready viewports, no JS window events or commits, and bounded mounted visuals. Check both dataset endpoints outside timing. |
| `scrollbar-drag-10000` | Hold the thumb a quarter of its height below its top and drag repeatedly down/up a 10,000-row native list using real pointer events. Measure thumb geometry update separately from the full ready-scene latency. Check the thumb follows the pointer without losing the grab offset, content moves in the same direction, rows stay ready/bounded, neither JS windows nor commits occur, both ends clamp correctly, and release stops dragging. |
| `rows-edit-100/1000` | Update a subtitle in a standard `Row` list with accented names and mixed one/two-line titles, as used by Reverb and Podcasts. Check the changed subtitle, neighbouring content, order and bounded mounted hosts. |
| `rows-prepend-1000`, `rows-reorder-1000` | Insert/remove before a scrolled viewport and reverse the collection. Check the visible anchor, supplied order and a native tap returning the current row's identity after reordering. |
| `rows-native-scroll-1000`, `rows-react-scroll-1000` | Scroll standard rows and custom nested track components. Both must show the correct moving content and retain bounded hosts. The native path produces no JS window work; the custom React path must actually exercise JS window materialisation. |
| `rows-decoded-image-arrival` | Feed authored decoded RGBA through the real native-image completion boundary. Check the artwork dimensions/pixels, scene image revision and neighbouring rows. This measures scene application only; it supplies the decoder's output and does not measure file reads, decoding or GPU upload. |
| `startup-runtime-to-library` | Start a fresh QuickJS runtime, React tree and engine, then reach a usable root screen. Source and icons are already read; this is not Android process launch or storage hydration. Every sample is a new runtime, including discarded warm-ups. |
| `navigation-open/back/tab` | Use native hit testing, native back and the compiled file router. Open an album with 100 rows, return to retained state and switch tabs. Check released page hosts, retained selection and valid back history. |
| `conversation-controlled-input` | Edit the real multiline native editor and apply the resulting controlled React acknowledgement. Check the final draft, UTF-16 cursor and retained hosts. Android IME delivery is outside this measurement. |
| `conversation-append/prepend` | Exercise `ConversationScreen`'s native message template, replies, author labels and reactions. Check new content, retained history, prepend anchor positions, receipt counts and bounded windows. |
| `conversation-input-background`, `conversation-input-under-load` | Schedule an edit every 10 ms and four incoming linked messages every 50 ms (80 messages/s), or every 5 ms (800 messages/s saturation stress), without waiting for completion. Measure prescribed input arrival through scene-applied JS acknowledgement, including late dispatch and queueing. Also record dispatch lateness, dispatch-to-acknowledgement time and native edit/event-dispatch time. Check that stale commits cannot overwrite newer typing, all batches arrive and the retained history contains exactly the newest 512 messages in order. |
| `playback-native-clock`, `playback-state-change` | Exercise the public `PlayingScreen`, native clock anchors and JS semantic track/state changes. Verify elapsed/duration labels and retained controls; native clock updates require no JS commit or layout measurement. An untimed real-clock observation checks a second boundary and that pausing stops animation work. Audio service/device response is outside this measurement. |
| `idle` | Observe a settled React scene for 250 ms. It must produce no commits, animation, scene revisions or new measurements. This is a host scene/runtime contract, not an Android frame-callback or battery measurement. |

The cells form a fixed grid using tiny text so every result can be inspected together. This retains the 500-cell plus resize workload class from the earlier optimisation work; it is a fresh fixture and its timings must not be compared directly with deleted historical benchmarks.

The 28 timed workloads cover common patterns found in Passes, Reverb, Beeper and Podcasts. They are controlled fixtures of those patterns, not timings of the complete apps: app-specific filtering, sorting, persistence, server latency and media-library scans remain separate concerns.

Update timing starts before host-message dispatch and ends after the resulting commit is applied to the native scene. It includes JS application work, React when applicable, QuickJS-to-Rust conversion, queueing and native apply/layout. It is not touch-to-display latency. Native apply is also recorded separately for the original cell/list update workloads. Native scrolling times cover Rust scrolling and window materialisation; custom-row samples also wait for React's window commits. Native `perf` instrumentation is enabled; full correctness checks, host-node diagnostics and commit serialisation run outside measured samples. Scene-readiness detection is part of the interaction measurement. Detailed bridge probes run separately from timing samples. Serialised commit bytes are a diagnostic representation, not measured physical wire bytes.

Loaded-input latency starts at the prescribed arrival time, rather than when the preceding operation finishes. The host interleaves scheduled arrivals and scene application without waiting for each acknowledgement. If React coalesces drafts, earlier drafts are counted as acknowledged when a newer committed draft supersedes them; the native editor must still retain the latest typed value. Per-commit editor checks add host work to this stress workload, so compare like-for-like baselines. These are input-to-application-acknowledgement timings, not the native editor's first visual response or physical input-to-display timings. Median, p95 and p99 are reported; short trial runs do not establish reliable tail distributions.

Load workloads begin with 128 messages and grow to a maximum of 512. Short trials may finish before reaching that full history, so their timings cannot be compared directly with longer runs. Baseline replay applies the selected sample/warm-up settings to both saved and candidate executables.

Samples alternate values so no-op setters cannot win. Warm-up operations are discarded. Each round starts a new process and runtime instances; raw operation samples, per-round medians, combined median/p95/p99 and work probes are saved. Build/preparation time is excluded from measured execution time. Host nodes and queue high-water bytes describe only those resources, not total RSS or all allocation/GC costs.

Scrollbar samples start immediately before each `pointer_move`. `thumb_update_ns` ends when the native scene's thumb rectangle has been computed through the same method used by the renderer; `elapsed_ns` also includes native row window materialisation and viewport readiness. Pointer capture and release are outside timing. The pointer remains held across repeated full-track sweeps, with independent checks that the thumb follows its requested position and that both endpoints remain reachable. These CPU timings exclude Android event delivery, frame scheduling, GPU submission and display presentation.

## Baselines and optimisation

Each successful run retains its release executable, exact compiled fixtures and hashes under ignored `.test-output/performance/`. `--baseline` reruns that saved executable with its own saved assets, alternating baseline/candidate order between rounds on the same host. Workload-source and machine mismatches reject comparisons; compiler or engine changes may produce different compiled assets. Raw per-round changes accompany aggregate percentages so noisy improvements remain visible.

Timing changes are reported without an automatic speed gate. Establish noise on the chosen host before introducing thresholds; a single fast run is not evidence of improvement. Structural contracts fail immediately. A dirty-tree source inventory and patch are recorded, but they are not a recoverable source snapshot. Commit source for reproducibility claims. Saved files do allow replaying the measured executable and fixtures.

The suite excludes Android scheduling/Vulkan, artwork decoding, playback services, storage/network IO, whole-process memory/energy and APK size. It does not benchmark drag/fling frame pacing or Canvas input. Those need their own production-path workloads; headless CPU results do not establish LP3 frame latency or power consumption. Keep the existing correctness and reviewed pixel suites alongside performance work.

Fixture bundles, release build caches and evidence remain ignored. Every runtime is stopped and joined when its workload finishes. No device packages, emulator processes or global device settings are created.

## Performance audit: 19 October 2026

The Android audit measured release APKs before and after adding QuickJS bytecode, direct Rust asset loading, retained Vulkan resources, Rust store/SQLite/encryption and faster compiled-list projection. A separate React host change removes the child-array search on append and clears parent references when removing a subtree.

The device fixture hydrates a store before rendering 5,000 standard rows. Both APKs ran on the Light Phone III API 34 emulator on an Apple M4 Pro. Each latency scenario used 10 samples; native operations used 10 warm-ups and 200 measured calls. The final APK was rebuilt after the native timeout and shutdown-cancellation fixes.

| Emulator workload | Before, median ms | After, median ms |
| --- | ---: | ---: |
| Process start to first submitted frame | 155.5 | 148.0 |
| Process start to hydrated content frame | 223.5 | 148.0 |
| Resume to submitted frame | 52.0 | 37.0 |
| One edit in 5,000 rows, input-up to submitted frame | 6.37 | 2.41 |
| Store read | 0.490 | 0.164 |
| Durable store write | 16.683 | 1.288 |
| Short AES-GCM encryption | 0.222 | 0.007 |
| Short AES-GCM decryption | 0.223 | 0.007 |
| SQLite query returning ten rows | 0.322 | 0.110 |

Process starts use `am force-stop`, retaining filesystem caches. Frame timings end at submission, before display presentation. Native timings include the public JavaScript SDK call and result delivery. Earlier alternating before/after blocks also showed faster resume, list edits and native calls; two separate cold-start comparisons with 30 samples per APK reduced hydrated-content latency by 91–92 ms. A single fresh block does not establish a precise first-frame improvement.

The headless suite reran the saved baseline and candidate in alternating order across three processes per version, with 20 warm-ups and 100 samples per workload/process:

| Headless workload | Before, median ms | After, median ms |
| --- | ---: | ---: |
| One edit in 100 items | 0.220 | 0.166 |
| One edit in 1,000 items | 0.743 | 0.207 |
| One edit in 10,000 items | 8.513 | 0.624 |
| One standard Row edit in 1,000 items | 0.794 | 0.283 |

These list results include QuickJS dispatch through native scene application. The remaining identity scan is O(n). Background-input results varied substantially: one comparison measured 0.760 → 1.081 ms, while a repeat of the same executables measured 0.953 → 0.782 ms. No background-input improvement or consistent regression is established by those runs. Other unchanged paths also show timing noise; retain every workload when reviewing the evidence.

A separate QuickJS compile-only benchmark on the 618,340-byte Podcasts bundle measured 29.87 ms to parse/compile source and 1.65 ms to read stripped bytecode, using 15 fresh runtimes. It excludes script execution, process launch and rendering. Icons avoid the Java byte-array transfer; their individual timing was not isolated from startup.

Appending 4,000 plain Text children to an existing React Stack measured 378.96 → 276.87 ms, with p95 386.23 → 285.67 ms. Before/after bundles used the corresponding production renderer sources and the same current `AppRuntime`. Two fresh runtimes per version ran in before/after/after/before order, with 10 warm-ups and 30 measured appends each. Timing spans dispatch through receipt of the React commit, whose 4,000 creates were checked after timing. It excludes native scene application and rendering; compiled virtualised Lists use a different path.

The fixture APK grew from 3,331,923 to 4,645,541 bytes, principally from bundled SQLite. SQLite is selected by the store capability. Retained renderer caches also keep resources alive while the activity is backgrounded; memory and energy changes were not measured. Background Vulkan preparation added waiting in the captured trace and was removed.

Evidence remains under `.test-output/performance-audit/2026-10-19/` and `.test-output/performance/2026-10-19T10-44-39.144Z-62261/`. It includes APKs, saved headless executables/assets, raw frame samples and logcat, native-operation summaries, fixture source, source hashes and screenshots. The saved screenshots match below the changing title, with zero differing pixels in `(0, 120)–(1080, 1240)`. These are emulator and host measurements; they do not establish Light Phone III hardware latency, whole-process memory or energy use.

## System SQLite follow-up: 19 October 2026

Android now uses the platform SQLite engine through a synchronous Kotlin backend on Rust database workers. Rust retains scheduling, query cancellation and direct runtime replies. Android's normal dependency tree contains neither `rusqlite` nor `libsqlite3-sys`; host fixtures retain the embedded backend. The store keeps its database path, schema, revision rules, WAL and full synchronisation. Available SQL features follow Android's SQLite version.

The same release fixture ran in bundled/system/bundled/system order on the same emulator. App bytecode, the imported database, icons and capability manifest are byte-identical. Generated DEX profiles changed with the backend code. Builds and correctness suites ran outside measurement blocks.

| Release fixture size | Bundled SQLite, bytes | System SQLite, bytes |
| --- | ---: | ---: |
| APK | 4,645,541 | 3,540,005 |
| Native library | 4,066,424 | 2,960,888 |
| DEX code | 398,612 | 406,824 |

The APK is 1,105,536 bytes smaller (23.8%). It remains 208,082 bytes larger than the original APK from before the performance audit.

| Emulator workload | Bundled SQLite, median ms | System SQLite, median ms |
| --- | ---: | ---: |
| Process start to first submitted frame | 152.5 | 158.0 |
| Process start to hydrated content frame | 152.5 | 162.0 |
| Resume to submitted frame | 34.0 | 34.5 |
| One edit in 5,000 rows, input-up to submitted frame | 2.557 | 2.603 |
| Store read | 0.1615 | 0.1885 |
| Durable store write | 1.2295 | 0.7820 |
| SQLite query returning ten rows | 0.1175 | 0.1285 |
| Short AES-GCM encryption | 0.0125 | 0.0070 |
| Short AES-GCM decryption | 0.0070 | 0.0070 |

Frame medians pool 20 raw samples per APK. Native values are the median of two block medians, each from 200 calls after 10 warm-ups; the per-call samples were not retained. Process starts retain filesystem caches, and frame timings stop at Vulkan submission rather than display presentation.

Writes were faster in both system-backend blocks: 1.287 → 0.761 ms and 1.172 → 0.803 ms. Their p95 values were 2.806 → 1.800 ms and 4.246 → 2.228 ms. This fixture also showed a startup cost: hydrated content took 9.5 ms longer at the pooled median. Reads added 0.027 ms and the ten-row query added 0.011 ms. Resume and list edits stayed close. Encryption code did not change; its differing observations do not establish an improvement from switching SQLite.

All 151 existing host checks, 11 Android app contracts and the SDK type check passed. Android native checks also passed for storage with background work, crypto-only background work and background work without either capability. The release benchmark exercises the system SQLite query path; the Android app contracts cover the system-backed store. No new tests were added.

Evidence remains under `.test-output/system-sqlite/`: both APKs and hashes, per-block summaries, raw frame samples and logcat, the fixture source, dependency tree and check logs. `summary.json` records the aggregation method and individual block medians and p95 values. These measurements do not establish physical Light Phone III latency, memory use or energy consumption.
