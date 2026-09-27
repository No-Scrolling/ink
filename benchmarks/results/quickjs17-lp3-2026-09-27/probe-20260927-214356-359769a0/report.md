# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.300 | 0.591 | 0.591 |
| React and transport | 15 | 10.672 | 24.057 | 24.057 |
| Json decode | 15 | 0.305 | 0.694 | 0.694 |
| Decode to apply | 15 | 0.052 | 0.112 | 0.112 |
| Apply and layout | 15 | 2.226 | 4.318 | 4.318 |
| Commit to frame submission | 15 | 2.378 | 5.036 | 5.036 |
| Input to frame submission | 15 | 15.964 | 34.515 | 34.515 |
| Commit to request | 15 | 0.095 | 0.192 | 0.192 |
| Request to frame start | 15 | 0.150 | 0.392 | 0.392 |
| Frame start to submission | 15 | 2.115 | 4.452 | 4.452 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
