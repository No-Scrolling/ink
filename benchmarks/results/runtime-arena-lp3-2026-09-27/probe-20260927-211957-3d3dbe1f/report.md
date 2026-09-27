# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11223 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.298 | 0.552 | 0.552 |
| React and transport | 15 | 13.268 | 29.000 | 29.000 |
| Json decode | 15 | 0.286 | 0.699 | 0.699 |
| Decode to apply | 15 | 0.037 | 0.109 | 0.109 |
| Apply and layout | 15 | 1.814 | 4.340 | 4.340 |
| Commit to frame submission | 15 | 2.036 | 4.983 | 4.983 |
| Input to frame submission | 15 | 17.111 | 38.514 | 38.514 |
| Commit to request | 15 | 0.071 | 0.178 | 0.178 |
| Request to frame start | 15 | 0.145 | 0.303 | 0.303 |
| Frame start to submission | 15 | 1.834 | 4.503 | 4.503 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
