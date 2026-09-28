# Headless React-to-scene benchmark

CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android input delivery, UI scheduling, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown. Native bindings fixture: React mounts the page; measured value updates bypass React reconciliation. This is not the unchanged React workload.

Measured updates: 200; discarded warm-up updates: 20.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.122 | 0.120 | 0.155 |
| decode | 0.000 | 0.000 | 0.000 |
| inputToDispatch | 0.000 | 0.000 | 0.001 |
| inputToScene | 0.277 | 0.272 | 0.329 |
| javascriptAndTransport | 0.154 | 0.151 | 0.185 |

## Whole-suite breakdown

Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. Excludes process launch, argument parsing and final report serialisation/output. Hyperfine runs are separate measurements.

| Work | Total (ms) | Share |
| --- | ---: | ---: |
| cellValidation | 4.486 | 5.3% |
| compatibilityChecks | 0.766 | 0.9% |
| harnessOverhead | 0.054 | 0.1% |
| measuredApplyAndLayout | 24.362 | 28.8% |
| measuredDecode | 0.000 | 0.0% |
| measuredInput | 0.081 | 0.1% |
| measuredJavascriptAndTransport | 30.859 | 36.5% |
| mountAndSetup | 14.256 | 16.9% |
| shutdown | 0.583 | 0.7% |
| warmupUpdates | 9.014 | 10.7% |
| **Total accounted** | **84.462** | **100%** |

Checks: all 500 labels on every update, row gap alternation, thumb drag, content drag, reverse, hide/show, count and scrollbar change.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/.agent-tools/headless-20260928-220853-219eccf7/ink-headless --assets /Users/vandam/Developer/ink/.agent-tools/headless-20260928-220853-219eccf7/assets --iterations 200 --warmup 20
```

Hyperfine whole-process mean: 0.079 s ± 0.000 s (standard deviation).
