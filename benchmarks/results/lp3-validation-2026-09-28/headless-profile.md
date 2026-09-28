# Headless React-to-scene benchmark

Host CPU only. JS includes real queue waits, React and native batch conversion. Decode is zero for native batches; commitBytes is unavailable without a JSON document. Total starts at native pointer-up and ends after scene layout; excludes validation, Android, GPU and presentation. Hyperfine additionally includes startup, warm-up, validation and shutdown. React diagnostic instrumentation is enabled; use uninstrumented runs for speed comparisons.

Measured updates: 200; discarded warm-up updates: 20.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.089 | 0.090 | 0.100 |
| decode | 0.000 | 0.000 | 0.000 |
| inputToDispatch | 0.000 | 0.000 | 0.000 |
| inputToScene | 1.485 | 1.479 | 1.520 |
| javascriptAndTransport | 1.395 | 1.390 | 1.429 |

## Whole-suite breakdown

Accumulated wall time in this instrumented run, from loading assets through runtime shutdown. Excludes process launch, argument parsing and final report serialisation/output. Hyperfine runs are separate measurements.

| Work | Total (ms) | Share |
| --- | ---: | ---: |
| cellValidation | 4.186 | 1.2% |
| compatibilityChecks | 9.707 | 2.7% |
| harnessOverhead | 0.304 | 0.1% |
| measuredApplyAndLayout | 17.882 | 5.0% |
| measuredDecode | 0.000 | 0.0% |
| measuredInput | 0.071 | 0.0% |
| measuredJavascriptAndTransport | 278.997 | 77.4% |
| mountAndSetup | 16.774 | 4.7% |
| shutdown | 0.592 | 0.2% |
| warmupUpdates | 31.910 | 8.9% |
| **Total accounted** | **360.424** | **100%** |

Checks: all 500 labels on every update, row gap alternation, thumb drag, content drag, reverse, hide/show, count and scrollbar change.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/.agent-tools/headless-20260928-112958-d7b21438/ink-headless --assets /Users/vandam/Developer/ink/.agent-tools/headless-20260928-112958-d7b21438/assets --iterations 200 --warmup 20 --react-profile
```

## Diagnostic React timings

Instrumented attribution only; use uninstrumented runs for speed comparisons. Component time is inside render time; JSX estimates are inside component time; host-update estimates are inside commit time. These must not be added together. Raw samples are in result.json.

| Measurement | Mean |
| --- | ---: |
| clockPairMs | 0.000042 |
| commitMs | 0.263162 |
| componentCalls | 2.000000 |
| componentMs | 0.309794 |
| effectsMs | 0.047040 |
| hostUpdateCalls | 0.000000 |
| hostUpdateMeasuredMs | 0.000000 |
| hostUpdateSamples | 0.000000 |
| jsxCalls | 0.000000 |
| jsxMeasuredMs | 0.000000 |
| jsxSamples | 0.000000 |
| nativeTransferMs | 0.042606 |
| renderMs | 1.029864 |
| workLoopMs | 1.015040 |
| workUnits | 0.000000 |
| reconcilerWorkMs | 0.705245 |
| beforeCommitOverheadMs | 0.014825 |
