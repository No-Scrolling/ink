# Headless React-to-scene benchmark

CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android input delivery, UI scheduling, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown.

Measured updates: 100; discarded warm-up updates: 10.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.101 | 0.097 | 0.125 |
| decode | 0.000 | 0.000 | 0.000 |
| inputToDispatch | 0.001 | 0.000 | 0.001 |
| inputToScene | 1.599 | 1.570 | 1.781 |
| javascriptAndTransport | 1.497 | 1.471 | 1.675 |

## Whole-suite breakdown

Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. Excludes process launch, argument parsing and final report serialisation/output. Hyperfine runs are separate measurements.

| Work | Total (ms) | Share |
| --- | ---: | ---: |
| cellValidation | 2.174 | 1.0% |
| compatibilityChecks | 10.488 | 5.0% |
| harnessOverhead | 0.028 | 0.0% |
| measuredApplyAndLayout | 10.055 | 4.8% |
| measuredDecode | 0.000 | 0.0% |
| measuredInput | 0.058 | 0.0% |
| measuredJavascriptAndTransport | 149.738 | 72.0% |
| mountAndSetup | 17.346 | 8.3% |
| shutdown | 0.653 | 0.3% |
| warmupUpdates | 17.533 | 8.4% |
| **Total accounted** | **208.072** | **100%** |

Checks: all 500 labels on every update, row gap alternation, thumb drag, content drag, reverse, hide/show, count and scrollbar change.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212559-960f4f67/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212559-960f4f67/assets --iterations 100 --warmup 10
```

