# Native bindings: declare controls once, update values

2026-09-28. Implemented `ink/view`, migrated Counter, added a template compatibility page and a 500-cell binding benchmark. Existing React applications retain their adapter. Rust now owns binding targets and persistent control updates; application decisions and calculations remain JavaScript. Existing PlayingScreen elapsed-time labels now use the native playback clock.

## Results

| Measurement, 500 cells + resize | React | Native bindings |
| --- | ---: | ---: |
| Mac input → scene mean | 1.628 ms | **0.267 ms** |
| Mac input → scene p95 | 1.763 ms | **0.292 ms** |
| Hyperfine whole process mean | 393.7 ms | **80.4 ms** |
| LP3 input → submission mean | 17.433 ms | **7.404 ms** |
| LP3 input → submission median | 13.494 ms | **6.854 ms** |
| LP3 input → submission p95 | 27.188 ms | **10.408 ms** |

Mac updates take 83.6% less time. LP3 mean submission latency falls 57.5%. This changes the update model; it does **not** establish that unchanged React reconciliation reached the old 1.5 ms goal. Submission is not physical display response.

### Mean waterfall

| Stage | Mac React | Mac bindings | LP3 React | LP3 bindings |
| --- | ---: | ---: | ---: | ---: |
| Native input → dispatch | 0.0005 | 0.0004 | 0.324 | 0.338 |
| JavaScript and transport | 1.5300 | 0.1489 | 13.348 | 2.742 |
| Dispatch after conversion → apply | included above | included above | 0.059 | 0.066 |
| Rust apply and layout | 0.0975 | 0.1179 | 0.917 | 1.401 |
| Commit → frame submission | excluded | excluded | 2.785 | 2.857 |
| **Total (ms)** | **1.6280** | **0.2672** | **17.433** | **7.404** |

The improvement comes from removing repeated JS element construction, reconciliation and host-instance comparison. Rust does a little more work resolving binding targets. This is a useful trade-off, not a claim that every stage became faster.

## Method

Mac: eight alternating pairs, 200 updates after 20 warm-ups per process, 1,600 measured updates per model. Both bundles ran on the **same current native binary**, with identical normal QuickJS-ng 0.17. No React/interpreter profiling instrumentation. The prepared builds' original timings overlapped build activity; use the quiet paired data here instead. Hyperfine ran separately: five processes per model after two warm-ups. Process times include startup, checks and shutdown; individual update timers do not.

LP3: candidate/baseline/baseline/candidate installation order. Each installation had one 15-tap warm-up and two 15-tap measured probes. Sixty accepted samples per model, zero rejected samples. Both APKs use `INK_BRIDGE_TIMING=1`. Device scheduling was unmodified. Baseline is the retained React build `experiment-20260928-130226-5dc27900`; candidate is `experiment-20260928-142015-a2fbedcb`. Baseline variability is retained, including the slow run. Candidate per-run p95s: 10.724, 9.851, 10.765 and 11.986 ms. Pooled p95 is calculated from raw samples, not averaged percentiles.

Every timed update changes all 500 labels and alternates all 50 row gaps. The binding fixture has a fixed tree: count changes hide predeclared rows/cells, and reverse changes bound label content. The React fixture uses its existing reconciliation and Activity path. Neither timed workload virtualises away cells. Their dynamic-structure semantics are not interchangeable.

## Correctness

- Both headless models passed all 500 labels and cell positions on every update, row-gap alternation, thumb dragging, content dragging, reversing, hide/show, and count/scrollbar changes.
- The real QuickJS runtime passed action dispatch, batching, Activity hide/reveal, unmount, stale callbacks and remount checks. Captured JS operations exactly matched the Rust protocol batches.
- Applying those commits to the native engine produced visible values 0 → 1 → 3 → hidden → 9 → removed → 20. The last mount changes its source synchronously inside onMount; no intermediate 0 is displayed. React emitted two harmless hidden commits. Reveal combines visibility and latest values in one commit.
- A focused native clock check verified 0:00 → 0:01 without JavaScript, seek to 1:05, and pause with no active playback animation.
- The **current** LP3 Counter APK incremented to Count: 1.
- The **current** LP3 template page passed increment, Japanese/combined-emoji rendering, controlled TextInput and echo, native progress advancement, pause, seek, navigation into ordinary React Inputs, ordinary input editing, and returning with count/input/position retained. Screenshots are alongside this report.
- Rust checks, Ink TypeScript checks, all three app TypeScript checks and `git diff --check` passed.

The template log contained Android's startup 4×4 AHardwareBuffer capability-probe errors. There were no Ink runtime errors or crashes during these checks. This was a focused compatibility pass, not an exhaustive test of every template module or network/audio service.

## Size

| Instrumented arm64 artefact | React baseline | Binding candidate | Change |
| --- | ---: | ---: | ---: |
| Native library | 2,480,384 B | 2,549,888 B | +69,504 B |
| APK | 2,661,636 B | 2,726,892 B | +65,256 B |
| App JS, uncompressed | 162,164 B | 149,146 B | −13,018 B |

This implementation improves speed and ownership, with a roughly 68 KiB native-library cost. It does not meet the earlier sub-2 MB bundle aspiration. The Counter APK is 2,732,116 bytes. Isolated test application IDs were used; production app data was not overwritten.

## Scope and follow-through

Persistent controls, property bindings, batching, lifetime integration and native playback labels are implemented. Keyed native repeaters, dynamic insertion and whole-player composite construction are **not** implemented. Existing dynamic React lists remain on the existing List adapter. The next architectural investigation should give a keyed list one native owner for row identity and viewport state, using this value transport. Do not turn fixed preallocated cells into a general list workaround.

See [native binding ownership and lifecycle](../../../../docs/native-bindings.md) for the interface contract. Sources are `packages/ink/src/view.ts`, shared scheduling/actions in `host.ts`, the React lifetime integration in `renderer.ts`, the protocol/runtime conversion, and native target expansion in `crates/ink-core/src/react/view.rs`.

## Evidence and repeat

- `mac-paired.json.gz`: all sixteen process reports and per-update durations.
- `app-final.json.gz`: all twelve probes, including separate warm-ups and native timing markers.
- `hyperfine.json`: independent whole-process comparison.
- `headless-provenance.json`: both preparation records, source hashes, QuickJS inputs and binary/bundle hashes.
- `manifest.json`: candidate APK hashes and snapshot hash. Agent-tools retains the immutable source archive and APKs.
- `lifecycle.tsx` / `lifecycle.json`: captured temporary fixture and actual protocol output. Fixture imports reflect its original `.agent-tools/native-views/` location.

```sh
scripts/agent-tools headless --app benchmarks/apps/ink-updates --background
scripts/agent-tools headless --app benchmarks/apps/ink-views --hyperfine --background
```

Run benchmarks without concurrent builds. Prepared commands in the provenance file run in a fraction of a second without compilation. Use the same candidate binary and each saved assets directory for a controlled adapter comparison.
