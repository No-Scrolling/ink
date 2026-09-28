# Headless React-to-scene benchmark

CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android input delivery, UI scheduling, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown. React diagnostic instrumentation is enabled; use uninstrumented runs for speed comparisons.

Measured updates: 200; discarded warm-up updates: 20.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.105 | 0.102 | 0.136 |
| decode | 0.000 | 0.000 | 0.000 |
| inputToDispatch | 0.001 | 0.001 | 0.002 |
| inputToScene | 1.684 | 1.667 | 1.817 |
| javascriptAndTransport | 1.578 | 1.565 | 1.707 |

## Whole-suite breakdown

Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. Excludes process launch, argument parsing and final report serialisation/output. Hyperfine runs are separate measurements.

| Work | Total (ms) | Share |
| --- | ---: | ---: |
| cellValidation | 4.561 | 1.1% |
| compatibilityChecks | 11.660 | 2.9% |
| harnessOverhead | 0.558 | 0.1% |
| measuredApplyAndLayout | 21.086 | 5.2% |
| measuredDecode | 0.000 | 0.0% |
| measuredInput | 0.142 | 0.0% |
| measuredJavascriptAndTransport | 315.654 | 77.5% |
| mountAndSetup | 17.101 | 4.2% |
| shutdown | 0.776 | 0.2% |
| warmupUpdates | 35.642 | 8.8% |
| **Total accounted** | **407.180** | **100%** |

Checks: all 500 labels on every update, row gap alternation, thumb drag, content drag, reverse, hide/show, count and scrollbar change.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/.agent-tools/headless-20260928-221045-6aba7a24/ink-headless --assets /Users/vandam/Developer/ink/.agent-tools/headless-20260928-221045-6aba7a24/assets --iterations 200 --warmup 20 --react-profile
```

## Diagnostic React timings

Instrumented attribution only; use uninstrumented runs for speed comparisons. Component time is inside render time; JSX estimates are inside component time; host-update estimates are inside commit time. These must not be added together. Raw samples are in result.json.

| Measurement | Mean |
| --- | ---: |
| clockPairMs | 0.000042 |
| commitMs | 0.327992 |
| componentCalls | 2.000000 |
| componentMs | 0.348820 |
| effectsMs | 0.053005 |
| hostUpdateCalls | 555.000000 |
| hostUpdateMeasuredMs | 0.006693 |
| hostUpdateSamples | 17.330000 |
| jsxCalls | 0.000000 |
| jsxMeasuredMs | 0.000000 |
| jsxSamples | 0.000000 |
| nativeTransferMs | 0.050874 |
| renderMs | 1.132273 |
| workLoopMs | 1.115328 |
| workUnits | 0.000000 |
| reconcilerWorkMs | 0.766509 |
| beforeCommitOverheadMs | 0.016945 |
| estimatedHostUpdateMs | 0.214353 |
| estimatedHostUpdateMinusClockMs | 0.191043 |
