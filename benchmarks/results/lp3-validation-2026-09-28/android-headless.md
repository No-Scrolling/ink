# Headless React-to-scene benchmark

CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android input delivery, UI scheduling, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown. Runs as Android shell, not an Activity; app scheduling and frame submission are not measured.

Measured updates: 100; discarded warm-up updates: 20.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.618 | 0.614 | 0.681 |
| decode | 0.000 | 0.000 | 0.000 |
| inputToDispatch | 0.006 | 0.005 | 0.006 |
| inputToScene | 7.431 | 7.480 | 7.724 |
| javascriptAndTransport | 6.807 | 6.854 | 7.090 |

## Whole-suite breakdown

Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. Excludes process launch, argument parsing and final report serialisation/output. Hyperfine runs are separate measurements.

| Work | Total (ms) | Share |
| --- | ---: | ---: |
| cellValidation | 10.279 | 1.0% |
| compatibilityChecks | 47.491 | 4.6% |
| harnessOverhead | 2.860 | 0.3% |
| measuredApplyAndLayout | 61.802 | 6.0% |
| measuredDecode | 0.000 | 0.0% |
| measuredInput | 0.585 | 0.1% |
| measuredJavascriptAndTransport | 680.700 | 66.0% |
| mountAndSetup | 81.277 | 7.9% |
| shutdown | 3.595 | 0.3% |
| warmupUpdates | 143.013 | 13.9% |
| **Total accounted** | **1031.602** | **100%** |

Checks: all 500 labels on every update, row gap alternation, thumb drag, content drag, reverse, hide/show, count and scrollbar change.

Run the prepared binary again:

```sh
scripts/agent-tools headless --serial LP3LHMA531900140 --token RESERVATION_TOKEN --ndk /Users/vandam/Library/Android/sdk/ndk/29.0.14206865 --iterations 100 --warmup 20
```

