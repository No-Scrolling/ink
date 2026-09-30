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
| `bindings-small`, `react-small` | Change one cell among 500 persistent controls. Verify every authored cell, unchanged neighbouring content and stable host-node count, with no layout measurement or full scene rebuild. The bound path transfers one source value and creates/removes no controls. |
| `bindings-bulk`, `react-bulk` | Change all 500 cells. Verify all 500 strings after every operation, in order, with no layout measurement or full scene rebuild. Bound changes batch 500 sources and retain mounted controls. |
| `bindings-resize`, `react-resize` | Change all 500 labels and their widths together. Check strings, proportional width changes and that cells remain inside the vertical clip. Bound changes batch 501 sources. |
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
| `idle` | Observe a settled bound scene for 250 ms. It must produce no commits, animation, scene revisions or new measurements. This is a host scene/runtime contract, not an Android frame-callback or battery measurement. |

The cells form a fixed grid using tiny text so every result can be inspected together. This retains the 500-cell plus resize workload class from the earlier optimisation work; it is a fresh fixture and its timings must not be compared directly with deleted historical benchmarks.

The 31 timed workloads cover common patterns found in Passes, Reverb, Beeper and Podcasts. They are controlled fixtures of those patterns, not timings of the complete apps: app-specific filtering, sorting, persistence, server latency and media-library scans remain separate concerns.

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
