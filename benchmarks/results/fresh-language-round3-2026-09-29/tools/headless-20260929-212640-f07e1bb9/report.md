# Headless React-to-scene benchmark

CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android input delivery, UI scheduling, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown. Native bindings fixture: React mounts the page; measured value updates bypass React reconciliation. This is not the unchanged React workload.

Measured updates: 100; discarded warm-up updates: 10.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.237 | 0.229 | 0.283 |
| decode | 0.000 | 0.000 | 0.000 |
| inputToDispatch | 0.000 | 0.000 | 0.001 |
| inputToScene | 0.253 | 0.242 | 0.303 |
| javascriptAndTransport | 0.015 | 0.012 | 0.032 |

## Whole-suite breakdown

Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. Excludes process launch, argument parsing and final report serialisation/output. Hyperfine runs are separate measurements.

| Work | Total (ms) | Share |
| --- | ---: | ---: |
| cellValidation | 2.268 | 3.8% |
| compatibilityChecks | 1.004 | 1.7% |
| harnessOverhead | 0.025 | 0.0% |
| measuredApplyAndLayout | 23.737 | 39.8% |
| measuredDecode | 0.000 | 0.0% |
| measuredInput | 0.046 | 0.1% |
| measuredJavascriptAndTransport | 1.516 | 2.5% |
| mountAndSetup | 27.536 | 46.2% |
| shutdown | 0.731 | 1.2% |
| warmupUpdates | 2.709 | 4.5% |
| **Total accounted** | **59.572** | **100%** |

Checks: all 500 labels on every update, row gap alternation, thumb drag, content drag, reverse, hide/show, count and scrollbar change.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212640-f07e1bb9/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212640-f07e1bb9/assets --iterations 100 --warmup 10
```

