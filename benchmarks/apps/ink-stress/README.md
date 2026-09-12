# Ink stress test

A local library app exercises navigation, virtualised rows, image sizes and resource recovery. It has 500 rows, 24 distinct 1,024 × 1,024 JPEGs, two datasets and an album detail page. The artwork uses coloured bands and a five-bit identifier; no network or external services are required.

Each cycle opens an album, returns, performs a continuous five-second drag, scrolls further, opens another album, returns to the top, refreshes through a deliberate failure and successful retry, then changes the dataset in Settings. The new dataset changes images and subtitle layout. An idle interval ends the run.

## Build and run

From the repository root:

```sh
scripts/agent-tools experiment --working-tree \
  --app benchmarks/apps/ink-stress --env INK_BENCHMARK=1 --background
```

Wait for the returned experiment ID to complete, then run:

```sh
scripts/agent-tools stress --baseline BASELINE_ID \
  --serial SERIAL --cycles 10 --background
```

The runner requires a 1080 × 1240 display, macOS Vision or Tesseract, and a device without an existing installation of `com.vandam.benchmark.ink.stress`. It uses the shared agent-tools reservation and cleanup. Do not run another SurfaceFlinger TimeStats collector on the same device. Use the physical Light Phone III for performance comparisons. The emulator is suitable for checking the journey.

`--cycles` defaults to 10; `--idle-seconds` defaults to 10. The run stops at the first failure. Screenshots and OCR add overhead between actions, so this is repeated use, not a rapid-tap or overlapping-finger test.

## Compare a change

Keep the fixture unchanged, make the framework change, and build a candidate with the same instrumentation flags. Then:

```sh
scripts/agent-tools stress --baseline BASELINE_ID --candidate CANDIDATE_ID \
  --serial SERIAL --cycles 10 --rounds 3 --background
```

The runner alternates baseline/candidate order each round and starts a fresh app process for each run. It rejects different fixture sources or instrumentation flags. Source hashes and APK hashes are retained. Inspect per-cycle values as well as the summary; the report presents measurements without claiming statistical significance.

Normal imported images already use target-size decoding in `platform/android/native/src/android.rs`. This fixture can compare changes to that behaviour, including a controlled full-resolution variant, but its baseline must not be described as decoding every row at full resolution.

## Results

Use `scripts/agent-tools wait ID --timeout 30` and `scripts/agent-tools result ID --full`. The evidence directory contains `report.md`, `result.json` and a directory for each run.

| Evidence | Meaning |
| --- | --- |
| Visible-state checks | Expected page titles, row labels, refresh states and dataset labels must appear within a 10-second polling window. |
| Artwork checks | The image column must contain substantial colour. This catches missing artwork, but does not prove every thumbnail is correct. |
| Continuous scrolling | SurfaceFlinger frame count, dropped frames and presentation-interval p95/p99; process CPU time over the drag. No screenshots or OCR run inside this measurement. |
| Renderer | Frame, layout, preparation, upload and submission wall times; aggregate uploaded bytes and cache misses, including text. |
| Memory | PSS/RSS before the loop, after each cycle and after settling. Growth includes cache warm-up and does not by itself establish a leak. |
| Idle | CPU time and instrumented rendered-frame count during the final pause. |
| Failure | Action sequence, last screenshot, OCR, runtime logs and a failure screenshot/log dump. Partial cycle results remain available. |

The `observedAfterMs` field includes screenshot transfer and OCR. It is a correctness diagnostic, **not input-to-display latency** and cannot validate the 50 ms target. SurfaceFlinger dropped frames are not a complete jank count; histogram buckets cannot prove every 16.7 ms deadline was met. Renderer submission time includes waiting, not just CPU execution. Instrumentation adds overhead.

A successful run confirms this workload completed. It does not guarantee recovery from every cache limit, eliminate all freezes or establish a physical-phone performance target from emulator results.
