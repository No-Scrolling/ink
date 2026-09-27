# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.320 | 0.583 | 0.583 |
| React and transport | 15 | 25.399 | 28.991 | 28.991 |
| Json decode | 15 | 0.470 | 0.831 | 0.831 |
| Decode to apply | 15 | 0.069 | 0.090 | 0.090 |
| Apply and layout | 15 | 2.803 | 3.518 | 3.518 |
| Commit to frame submission | 15 | 3.270 | 3.880 | 3.880 |
| Input to frame submission | 15 | 32.610 | 36.633 | 36.633 |
| Commit to request | 15 | 0.123 | 0.162 | 0.162 |
| Request to frame start | 15 | 0.218 | 0.428 | 0.428 |
| Frame start to submission | 15 | 2.928 | 3.478 | 3.478 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
