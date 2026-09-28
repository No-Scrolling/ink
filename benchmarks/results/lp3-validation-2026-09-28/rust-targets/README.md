# Native update target lookup

Retained two small changes to `ReactTree`: reserve the affected-target set from the update batch size, and reuse borrowed parent nodes while finding a text or canvas update target. The API and update semantics are unchanged. See [the exact incremental patch](change.patch).

## Paired CPU measurements

Six processes per variant, in ABBA order repeated three times, with 20 warm-ups per process. Mac processes measured 200 updates; LP3 shell processes measured 100. All existing headless fixture checks passed.

| Mean duration | Baseline | Candidate |
| --- | ---: | ---: |
| Mac native apply and layout | 0.10135 ms | 0.09328 ms |
| Mac total CPU update | 1.61974 ms | 1.58181 ms |
| LP3 native apply and layout | 0.59732 ms | 0.57889 ms |
| LP3 total CPU update | 7.21554 ms | 7.16507 ms |

Native savings are approximately 8% on Mac and 3% on LP3. JavaScript timing also varied, although JavaScript was unchanged; do not attribute all of the total difference to this patch. Mac total p95 was 1.81308/1.72425 ms; LP3 shell p95 was 7.59464/7.58745 ms.

Prepared baseline/candidate binaries: Mac `headless-20260928-124758-52d0cae4` / `headless-20260928-125646-3d8e6b96`; Android `headless-20260928-114535-e22ca8e5` / `headless-20260928-125845-516528a9`. Paired raw results are retained beside this report.

## Full-app LP3 check

Candidate/baseline/baseline/candidate, each installation followed by one warm-up and two measured 15-update probes. All 60 samples per variant were accepted. All eight measured screenshots showed 500 cells, Resize on, and the expected 30/45 update count. There were no rejected bridge samples.

| Duration | Baseline | Candidate |
| --- | ---: | ---: |
| Input to frame submission, mean | 16.467 ms | 16.114 ms |
| Input to frame submission, median | 12.821 ms | 12.282 ms |
| Input to frame submission, p95 | 26.395 ms | 27.147 ms |
| Native apply and layout, mean | 0.906 ms | 0.872 ms |

This supports a small native-phase saving, not a dependable full-app latency improvement. The under-16 ms p95 goal remains unmet. The full-app measurement ends at software frame submission, not physical display response.

Baseline APK: `experiment-20260928-043239-d135372a`. Candidate: `experiment-20260928-130226-5dc27900`, SHA-256 `76393f03e9b21aa88abb0efa83987974ec2b95c8c9d33b805ab768aad656c770`. Both use `INK_BRIDGE_TIMING=1`. Candidate APK is 2,661,636 bytes, 80 bytes larger than baseline. Warm-up and measured raw probe results are in `app-paired.json.gz`; the pooled measured summary is `app-summary.json`.

## Compatibility

The rebuilt native scene checker exactly matched the previous golden outputs for 97 linked-text scenes, 13 batch scenes and 24 runtime scenes. These compare text runs, quads and hit-test events. Counts and results are in `compatibility.json`. The current benchmark APK was exercised on the phone; the full template was not rebuilt for this small patch. The earlier template checks in the parent report predate it.
