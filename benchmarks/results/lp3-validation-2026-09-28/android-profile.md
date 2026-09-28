# Headless React-to-scene benchmark

CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android input delivery, UI scheduling, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown. React diagnostic instrumentation is enabled; use uninstrumented runs for speed comparisons. Runs as Android shell, not an Activity; app scheduling and frame submission are not measured.

Measured updates: 100; discarded warm-up updates: 20.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.614 | 0.611 | 0.703 |
| decode | 0.000 | 0.000 | 0.000 |
| inputToDispatch | 0.006 | 0.005 | 0.006 |
| inputToScene | 7.272 | 7.384 | 7.523 |
| javascriptAndTransport | 6.652 | 6.747 | 6.880 |

## Whole-suite breakdown

Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. Excludes process launch, argument parsing and final report serialisation/output. Hyperfine runs are separate measurements.

| Work | Total (ms) | Share |
| --- | ---: | ---: |
| cellValidation | 10.147 | 1.0% |
| compatibilityChecks | 47.956 | 4.7% |
| harnessOverhead | 4.740 | 0.5% |
| measuredApplyAndLayout | 61.419 | 6.0% |
| measuredDecode | 0.000 | 0.0% |
| measuredInput | 0.581 | 0.1% |
| measuredJavascriptAndTransport | 665.197 | 65.4% |
| mountAndSetup | 76.494 | 7.5% |
| shutdown | 4.561 | 0.4% |
| warmupUpdates | 145.701 | 14.3% |
| **Total accounted** | **1016.796** | **100%** |

Checks: all 500 labels on every update, row gap alternation, thumb drag, content drag, reverse, hide/show, count and scrollbar change.

Run the prepared binary again:

```sh
scripts/agent-tools headless --serial LP3LHMA531900140 --token RESERVATION_TOKEN --ndk /Users/vandam/Library/Android/sdk/ndk/29.0.14206865 --iterations 100 --warmup 20 --react-profile coarse
```

## Diagnostic React timings

Instrumented attribution only; use uninstrumented runs for speed comparisons. Component time is inside render time; JSX estimates are inside component time; host-update estimates are inside commit time. These must not be added together. Raw samples are in result.json.

| Measurement | Mean |
| --- | ---: |
| clockPairMs | 0.000156 |
| commitMs | 1.278752 |
| componentCalls | 2.000000 |
| componentMs | 1.406626 |
| effectsMs | 0.210054 |
| hostUpdateCalls | 0.000000 |
| hostUpdateMeasuredMs | 0.000000 |
| hostUpdateSamples | 0.000000 |
| jsxCalls | 0.000000 |
| jsxMeasuredMs | 0.000000 |
| jsxSamples | 0.000000 |
| nativeTransferMs | 0.267067 |
| renderMs | 4.335858 |
| workLoopMs | 4.249096 |
| workUnits | 0.000000 |
| reconcilerWorkMs | 2.842470 |
| beforeCommitOverheadMs | 0.086763 |
