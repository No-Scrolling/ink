# Headless React-to-scene benchmark

CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android input delivery, UI scheduling, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown. React diagnostic instrumentation is enabled; use uninstrumented runs for speed comparisons.

Measured updates: 100; discarded warm-up updates: 10.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.096 | 0.093 | 0.110 |
| decode | 0.000 | 0.000 | 0.000 |
| inputToDispatch | 0.000 | 0.000 | 0.001 |
| inputToScene | 1.533 | 1.521 | 1.630 |
| javascriptAndTransport | 1.437 | 1.426 | 1.518 |

## Whole-suite breakdown

Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. Excludes process launch, argument parsing and final report serialisation/output. Hyperfine runs are separate measurements.

| Work | Total (ms) | Share |
| --- | ---: | ---: |
| cellValidation | 2.035 | 1.0% |
| compatibilityChecks | 10.044 | 5.0% |
| harnessOverhead | 0.165 | 0.1% |
| measuredApplyAndLayout | 9.562 | 4.8% |
| measuredDecode | 0.000 | 0.0% |
| measuredInput | 0.046 | 0.0% |
| measuredJavascriptAndTransport | 143.742 | 72.2% |
| mountAndSetup | 15.960 | 8.0% |
| shutdown | 0.633 | 0.3% |
| warmupUpdates | 16.837 | 8.5% |
| **Total accounted** | **199.024** | **100%** |

Checks: all 500 labels on every update, row gap alternation, thumb drag, content drag, reverse, hide/show, count and scrollbar change.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212754-4d684b42/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212754-4d684b42/assets --iterations 100 --warmup 10 --react-profile
```

## Diagnostic React timings

Instrumented attribution only; use uninstrumented runs for speed comparisons. Component time is inside render time; JSX estimates are inside component time; host-update estimates are inside commit time. These must not be added together. Raw samples are in result.json.

| Measurement | Mean |
| --- | ---: |
| clockPairMs | 0.000042 |
| commitMs | 0.280334 |
| componentCalls | 2.000000 |
| componentMs | 0.317856 |
| effectsMs | 0.050285 |
| hostUpdateCalls | 0.000000 |
| hostUpdateMeasuredMs | 0.000000 |
| hostUpdateSamples | 0.000000 |
| jsxCalls | 0.000000 |
| jsxMeasuredMs | 0.000000 |
| jsxSamples | 0.000000 |
| nativeTransferMs | 0.043785 |
| renderMs | 1.050632 |
| workLoopMs | 1.035505 |
| workUnits | 0.000000 |
| reconcilerWorkMs | 0.717648 |
| beforeCommitOverheadMs | 0.015127 |
