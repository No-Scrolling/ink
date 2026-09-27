# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.291 | 0.700 | 0.700 |
| React and transport | 15 | 16.011 | 24.126 | 24.126 |
| Json decode | 15 | 0.464 | 0.692 | 0.692 |
| Decode to apply | 15 | 0.069 | 0.123 | 0.123 |
| Apply and layout | 15 | 2.430 | 4.338 | 4.338 |
| Commit to frame submission | 15 | 2.904 | 4.991 | 4.991 |
| Input to frame submission | 15 | 21.353 | 34.325 | 34.325 |
| Commit to request | 15 | 0.134 | 0.194 | 0.194 |
| Request to frame start | 15 | 0.180 | 0.453 | 0.453 |
| Frame start to submission | 15 | 2.625 | 4.472 | 4.472 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
