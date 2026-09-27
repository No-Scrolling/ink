# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11223 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.327 | 0.564 | 0.564 |
| React and transport | 15 | 10.626 | 23.872 | 23.872 |
| Json decode | 15 | 0.283 | 0.724 | 0.724 |
| Decode to apply | 15 | 0.045 | 0.110 | 0.110 |
| Apply and layout | 15 | 1.718 | 4.743 | 4.743 |
| Commit to frame submission | 15 | 2.127 | 5.147 | 5.147 |
| Input to frame submission | 15 | 15.036 | 33.092 | 33.092 |
| Commit to request | 15 | 0.080 | 0.178 | 0.178 |
| Request to frame start | 15 | 0.170 | 0.335 | 0.335 |
| Frame start to submission | 15 | 1.904 | 4.650 | 4.650 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
