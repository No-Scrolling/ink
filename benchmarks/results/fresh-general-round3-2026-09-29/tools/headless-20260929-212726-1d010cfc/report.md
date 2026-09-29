# Headless React-to-scene benchmark

CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android input delivery, UI scheduling, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown. Native bindings fixture: React mounts the page; measured value updates bypass React reconciliation. This is not the unchanged React workload.

Measured updates: 20; discarded warm-up updates: 3.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.224 | 0.223 | 0.244 |
| decode | 0.000 | 0.000 | 0.000 |
| inputToDispatch | 0.000 | 0.000 | 0.001 |
| inputToScene | 0.236 | 0.235 | 0.259 |
| javascriptAndTransport | 0.012 | 0.011 | 0.016 |

## Whole-suite breakdown

Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. Excludes process launch, argument parsing and final report serialisation/output. Hyperfine runs are separate measurements.

| Work | Total (ms) | Share |
| --- | ---: | ---: |
| cellValidation | 0.489 | 1.5% |
| compatibilityChecks | 1.032 | 3.1% |
| harnessOverhead | 0.004 | 0.0% |
| measuredApplyAndLayout | 4.487 | 13.3% |
| measuredDecode | 0.000 | 0.0% |
| measuredInput | 0.009 | 0.0% |
| measuredJavascriptAndTransport | 0.231 | 0.7% |
| mountAndSetup | 25.951 | 77.0% |
| shutdown | 0.739 | 2.2% |
| warmupUpdates | 0.748 | 2.2% |
| **Total accounted** | **33.689** | **100%** |

Checks: all 500 labels on every update, row gap alternation, thumb drag, content drag, reverse, hide/show, count and scrollbar change.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-212726-1d010cfc/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-212726-1d010cfc/assets --iterations 20 --warmup 3
```

