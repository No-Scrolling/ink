# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11223 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.304 | 0.551 | 0.551 |
| React and transport | 15 | 10.301 | 23.793 | 23.793 |
| Json decode | 15 | 0.281 | 0.848 | 0.848 |
| Decode to apply | 15 | 0.044 | 0.083 | 0.083 |
| Apply and layout | 15 | 1.707 | 3.133 | 3.133 |
| Commit to frame submission | 15 | 2.135 | 3.904 | 3.904 |
| Input to frame submission | 15 | 14.820 | 31.423 | 31.423 |
| Commit to request | 15 | 0.081 | 0.135 | 0.135 |
| Request to frame start | 15 | 0.142 | 0.246 | 0.246 |
| Frame start to submission | 15 | 1.913 | 3.523 | 3.523 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
