# Audit fixes and fresh reviews — 29 September 2026

All three confirmed defects from the preceding general audit are corrected. Two fresh independent reviewers then reconstructed the current implementation without receiving the earlier reports or conclusions. Their investigation established two additional general defects and Proxy/callable-property compatibility gaps; these were corrected and rereviewed during the audit. Larger language-ownership proposals remain architectural work, with their required measurements recorded in the new language audit.

## Corrections

| Area | Result | Evidence and limits |
| --- | --- | --- |
| Store admission | Measure the complete escaped read-result envelope before persistence, reserving the widest safe request ID and future revision. | `store-wire-admission.log`: the original 400,000-slash case needs 1,600,118 bytes under the reserved envelope and is rejected; a small value fits. Uses actual installed Android 36.1 `org.json` implementation on the host, not phone SQLite. |
| Store recovery | `set` and `reset` perform an atomic replacement without reading the old value. `update` retains CAS retries. Native metadata-only reads retain newer-version protection; version and revision support safe JavaScript integers. | `store-sdk-after.jsonl`: actual SDK with a mock native adapter recovers after an explicitly failed read; reset calls replacement directly. Android compilation passes. |
| List end policy | Remove compiled-list initialisation's whole-screen scroll. Explicit screen/conversation end policy remains. | `list-end-after.log`: the saved compiler/React fixtures through the current Rust engine both retain scroll offset zero. Existing conversation/list fixtures pass. |
| Conversation file actions | Include file-handler availability in presentation captures, so adding/removing the handler updates native hit targets. | `conversation-after.jsonl`: actual renderer emits the changed preview action capability; current callback dispatch remains fresh. |
| Bound-list options | Treat absent `itemChanges` as an empty patch when updating an already mounted list. Supplied malformed patches remain errors. | Fresh general reviewer reproduced the supported values-only update failing before correction and passing afterwards, with an explicit-empty-patch control. |
| Stream expiry | Keep a pending sweep, use monotonic idle ages and schedule from the earliest retained deadline. Subsequent opens cannot postpone abandoned handles indefinitely. | Fresh general reviewer traced original and corrected scheduler/call sites. Android build passes; timed Android expiry remains unmeasured. |
| Proxy compatibility | QuickJS supplies a side-effect-free `JS_IsProxy` predicate. Eligibility rejects root, nested, captured and callable Proxies before reflection; other JS hosts retain conservative object-capture reuse. | Fresh language review's actual QuickJS controls match React's 32 visible reads/calls, with no added reflection traps. Plain records still compile. |
| Callable properties | Validate and snapshot ordinary function data properties as well as object fields. Accessors retain React; mutable captured data invalidates presentation. Only a bare function prototype's intrinsic constructor back-reference is exempted from cycle traversal. | All 16 final release QuickJS controls pass: getters match React's visible reads, changed own data transfers 1,000 `After` records, and declared/arrow/bound callbacks remain native. The general reviewer separately validates the compiler/helper mechanism and fallback controls. |
| Late native resources | Dispose newly created managed imports/prepared images when delivery loses ownership. Oversized result envelopes also dispose their resource-bearing result. | Source ownership review and Android compilation; no physical picker/cancellation experiment. |

## Removed work

- Compiler-proven immutable field keys reuse unchanged keys. In the actual QuickJS 1,000-record diagnostic, a one-record replacement performs one projection and one key call, transferring one record. The previous diagnostic called every key extractor.
- Captured plain data has shared structural snapshots and comparison memoisation. Callback-only state changes with the same object capture now perform zero row projections, key calls and transfers; the previous diagnostic projected every row. Changes to captured data still invalidate presentation, and actions use current callbacks.
- Native list rows share retained JSON records through `Arc`; bound collections copy only a changed shared record on mutation. Whole maps/order/version storage and query staging still have costs described by the language review.
- Rust timer deadlines have an ordered deadline index and cancellation lookup, replacing nearest/due scans. Binary queue admission maintains its byte count instead of summing every queued buffer.
- Fetch `arrayBuffer()` returns the freshly owned exact-sized buffer from `bytes()`, removing a second complete copy while preserving Blob snapshot ownership.

`quickjs-projection-final.jsonl` records operation counts through the actual runtime. These are operation reductions, not measured LP3 speedups. Superseded intermediate observations were removed during result cleanup.

## Verification and cleanup

- Existing workspace Rust tests: eight passed (`cargo-test-final.log`).
- Exact existing compiler/native Bun suites: 16 passed, 29 assertions (`bun-test-final.log`).
- Ink TypeScript and explicit store/network checks passed (`typescript-check-final.log`, `sdk-typescript-check.log`).
- Fresh existing list, native-controls and bound-view headless fixtures passed. After the callable-property correction, the reviewers' final list runs are `headless-20260929-214432-6c8220ea` and `headless-20260929-214434-43f4c0e8`. The reports retain their exact source fingerprints.
- Final isolated `examples/light-template` Android build: `experiment-20260929-214438-465c7f95`, APK SHA-256 `3308a1c39a2f064d0e6287343935a67d1dc4f9d059398c1c8eec24bcaa6be090`. It includes the callable-property correction. Comparing the built SDK file with current source shows only the final local-variable rename (`captured` to `field`); current-source type checks and reviewer runtime/headless controls cover that cleanup. No APK was installed.
- Saved screenshot check `image-20260929-211740-21e3d313` compared the native-controls region with zero changed pixels and OCR. Its inputs are retained as `native-controls-before.png` and `native-controls-after.png`; the superseded report directories were removed. No new visual defect or rendering appearance claim was inferred from previews.
- Applied the cleanup skill: simplified capture comparison, shared duplicated store publication, removed local snapshot-variable shadowing, preserved necessary boundary checks and avoided unrelated formatting/refactoring. Final `git diff --check` passes. No application or repository tests were added; standalone observations are evidence programmes.
- Final audit-input revalidation checks all 1,146 recorded source hashes against the live tree with zero mismatches (`final-audit-input-revalidation.json`).

The workspace already contained extensive changes. Duplicate patch dumps and superseded check logs were removed during result cleanup. The final logs, audit source fingerprints and semantic controls remain.

## Independent audits and remaining work

- [Fresh general audit](../fresh-general-round3-2026-09-29/README.md): both independently established defects corrected, plus independent validation of the supplied callable-property trigger; no further actionable defect established in the final reviewed source.
- [Fresh language audit](../fresh-language-round3-2026-09-29/report.md): responsibility map, end-to-end cost reconstruction, actual runtime controls and prioritised native-work proposals.

The language reviewer also investigated deep capture traversal. A standalone unoptimised Cargo dev diagnostic exhausts its stack at depth 12. Release and Ink's actual Android `ink-dev` profile both pass the tested depths through 24 and correctly fall back at depth 64. This is retained as a diagnostic profile limitation, not promoted to an application defect; no stack limit or traversal implementation was changed in response.

Automatic persistent subtree compilation, selective native projection IR, shared collection/list record handles, narrower query transactions, typed transport and expression slots, further native layout work, stream ownership and DSP migrations remain proposals. Their runtime benefit and compatibility require separate implementation and acceptance evidence. Hardware lifecycle, GPU presentation, timed Android expiry, physical services and memory soak behaviour were not measured this turn.
