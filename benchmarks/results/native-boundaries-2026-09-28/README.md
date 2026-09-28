# Native boundary changes — 28 September 2026

Implemented native playback clocks, native built-in list templates, incremental unchanged-order list patches and typed network byte transport. Existing public list/component calls remain compatible. Native playback is explicitly selected because dropping periodic `state.position` updates would change existing applications.

## Paired list measurements

Five alternating before/after rounds on the same Mac, 200 measured updates and 20 warm-ups per process. Both saved builds have identical fixture and harness hashes. Builds had finished before this comparison. The dataset contains 1,000 keyed records; one-row edits include JS/React, transport and Rust layout, not just patch application.

| Mean duration | Before | After | Change |
| --- | ---: | ---: | ---: |
| Standard Row, one edit | 1.517 ms | 1.218 ms | −19.7% |
| Text/Stack row, one edit | 1.498 ms | 1.291 ms | −13.8% |
| Standard Row, unrelated update | 1.036 ms | 1.008 ms | −2.7%; small difference |
| Standard Row, native scroll window | 0.086 ms | 0.083 ms | Small difference; no scroll improvement claim |

[Raw paired results](list-paired.json) retain per-run summaries and checks. Before: `headless-20260928-174539-a82d28af`; after: `headless-20260928-174952-5d0ec0bf`. These saved intermediate builds isolate the patch work more closely than the later fixture, which adds built-in pages/checks. Other binary-bridge code also changed between them, so these are end-to-end build comparisons, not an isolated native function timer.

The final expanded fixture passes bounded windows, stateful custom React rows, fresh callbacks, deletion, prepend/reverse anchoring, sparse record transfer, native reorder actions and native conversation end positioning/scrolling. Both added built-ins assert zero JS window callbacks. [Final results](headless-final.json), operation `headless-20260928-181816-c54cd87d`.

[Hyperfine](hyperfine.json) averaged **0.438 seconds** per prepared run across five repetitions, with 50 measured updates and five warm-ups. The ordinary 200-update run also passed (`headless-20260928-180548-ee30a88f`, 1.452 seconds per process). These are Mac CPU checks, excluding Android scheduling, GPU rendering and panel latency. Whole-process times include setup and correctness checks; do not divide them into tap latency.

## Emulator validation

Used the visible Light Phone III emulator, Android 14, 1080 × 1240. The temporary package was `com.vandam.benchmark.ink.boundaries`; the existing template app was preserved. Builds used `INK_SPLIT_WEB=0`, `INK_BRIDGE_TIMING=1` and an isolated manifest with `network-cleartext` for the local server.

| Area | Result |
| --- | --- |
| Conversation appearance | Old React message rendering and final native rendering are pixel-identical at the same initial end position, including text, image/link preview and composer. |
| Conversation actions | Photo opening, retrying a failed message and long-press actions worked. |
| Reordering | Moving Item 0 down produced Item 1, Item 0, Item 2. Native scrolling and action routing also pass the 1,000-row headless check. |
| Media picker | Seeded 65 labelled images, crossed the 60-item page boundary, reached the final partial row and returned. Selected cells remained selected; only scrollbar geometry changed in the top-screen comparison. Selecting another recycled cell and confirming imported exactly the three selected fixture images. |
| Native playback | Bar and elapsed-time label advanced from 0:12 to 0:16 over four seconds with **0 ReactApply / 0 ReactReady** markers. |
| Existing React playback | Default `usePlayer` still produced **15 ReactApply / 15 ReactReady** markers in a four-second sample, with live position values. |
| Pause, seek and speed | Pause screenshots are identical over two seconds. Seeking reached 1:15. At 2×, three seconds advanced the label from 1:16 to 1:22. |
| HTTP download | 3,145,728 bytes, checksum 149,422,080. Chunk boundaries may vary. |
| Multipart upload | 1,048,576-byte file, all sampled bytes 37, UTF-8 form value “Ink café”. |
| Redirect replay | 1 MiB POST retained its body after HTTP 307; original Request was consumed. |
| WebSocket | UTF-8 text and `[0, 1, 255]` binary echoed; clean close code 1000. |
| Cancellation | Response cancelled after the first chunk; local server confirmed cancellation at chunk 1. |
| Background worker | Template's `example-todo` fetch returned success through the worker byte bridge and the screen reported ready. |

Conversation [before](conversation-before.png) / [final](conversation-final.png), [pixel comparison](image-20260928-181744-170835da.json). Action captures: [photo](conversation-photo.png), [retry](conversation-retry-after.png), [menu](conversation-menu.png). Reorder [before](reorder-before.png) / [after](reorder-after.png).

Media selection [before scrolling](media-selected.png) / [after returning](media-selected-return.png), [comparison](image-20260928-181235-6e0df17d.json), [imported fixtures](media-import.png). The comparison changed 470 pixels, all within the scrollbar bounds.

Playback [before](player-native-before.png) / [after](player-native-after.png), [native log](player-native.log), [default log](player-react.log), [pause comparison](image-20260928-181103-b6623abe.json), [seek](player-seek.png), 2× [before](player-2x-before.png) / [after](player-2x-after.png).

Network captures: [download](network-stream.png), [form](network-form.png), [redirect](network-redirect.png), [socket](network-socket.png), [cancel](network-cancel.png), [background status](background.png).

The conversation baseline is `experiment-20260928-180635-934fffd4`, with only `conversation.ts` replaced by its HEAD version. Other changes are present, so this is a visual composition comparison, not a whole-engine baseline. Broad emulator checks used `experiment-20260928-180647-1a499bd5`. The final build, `experiment-20260928-181610-c63f122d`, additionally checks binary request envelopes and preserves the ordinary image dimension limit; its launch and pixel comparison passed. APK hashes and source hashes are retained in the three manifest files beside this report.

The first launch exposed missing `@JvmStatic` annotations on JNI declarations. Those were fixed before the successful checks. Captures taken after failed launches were excluded because the previous app had regained focus. No successful-run Ink runtime errors were found. Test app, seeded media, port forwarding and server were removed; device reservation released. Isolated build workspaces were cleaned, retaining source archives and APK evidence.

## Regression checks and limits

- `cargo test -p ink-core -p ink-runtime --lib --quiet`: 8 passed. Includes sparse height invalidation across widths, native clock pause/speed/seek and byte ownership/limits.
- `bun test benchmarks/checks/native.test.ts crates/ink-compiler/src/native-lists.test.ts`: 15 passed. Includes abort/late replies, errors, exact managed Blob slice ranges and compiler compatibility.
- Root Ink and template TypeScript checks passed.
- Android native cross-check and complete Kotlin/Rust APK builds passed. The worker JNI path ran in the emulator.

Custom component hooks, effects and dynamic React trees retain their compatibility path. Conversation timestamp/link processing and application event logic remain JS. The network path removes base64 conversion but still makes ownership/JNI copies. No paired network throughput, APK-size or LP3 result is claimed here. LP3 was not connected; emulator results do not establish physical-device frame latency. Reverb's custom audio adapter does not automatically opt into `@ink/audio` native clocks.
