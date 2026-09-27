# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11223 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.313 | 0.385 | 0.385 |
| React and transport | 15 | 10.126 | 24.168 | 24.168 |
| Json decode | 15 | 0.261 | 0.676 | 0.676 |
| Decode to apply | 15 | 0.043 | 0.089 | 0.089 |
| Apply and layout | 15 | 1.752 | 3.740 | 3.740 |
| Commit to frame submission | 15 | 2.095 | 3.880 | 3.880 |
| Input to frame submission | 15 | 14.317 | 32.791 | 32.791 |
| Commit to request | 15 | 0.080 | 0.152 | 0.152 |
| Request to frame start | 15 | 0.142 | 0.244 | 0.244 |
| Frame start to submission | 15 | 1.872 | 3.484 | 3.484 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
