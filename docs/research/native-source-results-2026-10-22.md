# Native source performance results

Ink now provides an opt-in `createSource` and `SourceList` path. Rust owns keyed records and immutable snapshots, maintains equality-filtered views, and sends changes directly to native row templates. React can subscribe to a view's count with `useSnapshot`. Existing array-based lists keep their API.

For the 5,000-record inbox fixture, a host edit fell from **0.572 ms to 0.131 ms**. On the Light Phone III emulator, the complete edit fell from **2.200 ms to 1.689 ms**. A complete replacement also improved. Android edit p95 increased slightly, so this run does not establish a tail-latency improvement.

## Before and after

Each edit changes a visible message subtitle and its unread membership. Each replacement regenerates every message with a new subtitle. Both versions must display the same records and unread count before an operation is complete.

| Environment and operation | Before median | After median | Reduction |
| --- | ---: | ---: | ---: |
| Host: edit one of 1,000 records | 0.294 ms | 0.132 ms | 55% |
| Host: replace 1,000 records | 9.677 ms | 3.737 ms | 61% |
| Host: edit one of 5,000 records | 0.572 ms | 0.131 ms | 77% |
| Host: replace 5,000 records | 49.161 ms | 18.313 ms | 63% |
| Android emulator: edit one of 5,000 records | 2.200 ms | 1.689 ms | 23% |
| Android emulator: replace 5,000 records | 51.025 ms | 23.794 ms | 53% |

Host timing starts at command dispatch and ends when both the changed subtitle and unread count are applied to a ready native scene. It includes JavaScript, React where used, queueing, bridge conversion and native application/layout. Android timing starts at input-up and ends at the first submitted frame containing the final scene commit for that operation. Display presentation and physical e-paper response are excluded.

| Android operation, 5,000 records | Before p95 | After p95 |
| --- | ---: | ---: |
| Edit one record | 3.458 ms | 3.668 ms |
| Replace all records | 56.107 ms | 25.713 ms |

The edit median improved in both Android blocks. The 0.210 ms p95 increase remains a limitation of these results. Source-bound lists publish together, but a React count subscription can arrive in a separate commit; sparse host edits used two commits instead of one. This implementation does not guarantee atomic presentation across native lists and arbitrary React subscribers.

## What changed

- `ink-protocol::source::Graph` stores keyed JSON records in a persistent tree. Sparse edits copy changed paths. Equality-filtered membership uses subtree counts, so reading or updating a count does not flatten the record order.
- Immutable snapshots retain their original records after later transactions or release. Unchanged record and view revisions are preserved. Invalid transactions reject without mutation.
- The runtime installs synchronous source and snapshot APIs. Native list hosts bind to view IDs, including rebinding and descendant cleanup. Source publications use the existing bounded commit queue and fail closed on delivery failure.
- Native lists share snapshot records and patch changed row versions. Stable edits retain key order and indices. Initial mounting, structural membership changes and revision recovery can rebuild list metadata.
- The SDK exposes `createSource`, `set`, `transact`, `replace`, equality views, `getSnapshot`, `subscribe`, `dispose`, and `SourceList`. Field bindings use strings rather than JavaScript row projections. The Source API is documented in `docs/data.mdx`.

This is the first native data/view slice, not a general query engine. It does not include sorting, joins, arbitrary predicates, persistence, automatic migration of existing apps or a new JavaScript compiler. Applications adopt it explicitly for suitable collections.

## Validation

All 50 existing SDK checks and 47 runtime/scene checks passed. Four existing native-list checks also passed. Eight additional throwaway production-path probes verified coalesced mount/rebind/removal, descendant cleanup and reparenting. No unit tests were added.

The source benchmark uses real QuickJS, the production SDK, native hit testing, row event routing, `ReactTree` and `Engine`. Untimed checks cover retained snapshots, frozen records, stale/duplicate/unknown-key rejection, no-op notification silence, multiple filter fields, missing versus null values, membership crossings, replacement order, disposed snapshots, and current record selection after edits and reversals. All timed runs passed these checks. The materialised scene remained bounded at 47 host nodes and 18 row titles in both paths.

Six saved before/after Android screenshot pairs cover initial content, 48 edits and 12 subsequent full replacements, across both block pairs. Every pair is 1080 × 1240. Decoded RGBA comparison over the entire frame found **zero changed pixels** in every pair.

The 30 existing performance workloads and idle contract passed in four alternating control processes. Ordinary 1,000-row edits measured 0.275 ms before and 0.276 ms after; ordinary startup measured 8.868 ms and 8.904 ms. Some controls were slightly slower: the 500-control small edit measured 1.345 ms and 1.401 ms, a 4.2% increase. These controls do not establish a universal improvement outside the opt-in source path.

The APK increased from 3,429,082 to 3,554,122 bytes: **125,040 bytes, or 3.6%**. Total process memory, energy, physical-device latency and frame pacing were not measured.

## Method and evidence

The native baseline is `dfef70ed9f2d5564914c868ff332eaa98c288b83`. The candidate is an uncommitted source snapshot. Final production files were checked against the measured build's hashes. Both Android APKs contain identical JavaScript and identical QuickJS bytecode; their native libraries differ. The Android baseline SDK received only the new JavaScript exports/modules so it could compile the shared feature-detecting fixture. The headless baseline used a separate clean Git archive with only the identical harness and binary registration added.

All builds finished before timing. The host was an Apple M4 Pro, arm64 macOS. Host source measurements used four fresh processes in baseline/candidate/candidate/baseline order, 12 discarded warm-ups and 64 measured operations per workload/process: 128 samples per version/workload. No Bun execution substitutes for QuickJS.

Android measurements used release APKs on the Light Phone III API 34 emulator, launched with `emulator -avd Light_Phone_III -writable-system`. Four fresh app processes ran in the same alternating order. Each discarded eight edit warm-ups and measured 40 edits, then discarded two replacement warm-ups and measured ten replacements: 80 edits and 20 replacements per version. Logs identify every input, commit and submitted scene. The endpoint uses the last commit before the next input, including the separate count commit. Saved final pixels verify the resulting visible states.

The fixture compares 1,000 and 5,000 records because the array baseline's full row payload at 10,000 records exceeds its existing 1 MiB commit input limit. The results exclude file reads, production network/storage work, Android process launch and display presentation. Short tail distributions are descriptive, not latency guarantees.

Evidence is saved under `.test-output/native-source/2026-10-22/`:

| File | Contents |
| --- | --- |
| `paired/result.json`, `paired/report.md` | Host distributions and comparison |
| `paired/*-baseline.json`, `paired/*-candidate.json` | Raw host samples, setup observations and contract results |
| `paired/source/`, `paired/build.json` | Source snapshots, hashes, toolchains and build identity |
| `paired/bundle/` | Both saved release executables and shared assets |
| `device-summary.json`, `device-raw.json` | Android distributions and every sample |
| `device-*.log`, `device-*.png` | Input/commit/submission logs and saved screenshots |
| `pixel-comparison.json` | All six decoded pixel comparisons |
| `controls/` | Existing workload replay and raw controls |
| `evidence-manifest.json` | 66 artefact hashes, APK/bytecode identity and final-source verification |

`test/performance/source-run.ts` builds and replays the headless comparison. `test/performance/README.md` describes its options and baseline archive requirements. Android build and replay scripts are retained with the experiment evidence.
