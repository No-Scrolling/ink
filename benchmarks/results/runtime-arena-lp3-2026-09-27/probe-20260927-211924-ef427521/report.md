# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.306 | 0.590 | 0.590 |
| React and transport | 15 | 10.138 | 24.711 | 24.711 |
| Json decode | 15 | 0.287 | 0.700 | 0.700 |
| Decode to apply | 15 | 0.047 | 0.168 | 0.168 |
| Apply and layout | 15 | 1.731 | 4.565 | 4.565 |
| Commit to frame submission | 15 | 2.032 | 5.114 | 5.114 |
| Input to frame submission | 15 | 14.652 | 34.316 | 34.316 |
| Commit to request | 15 | 0.079 | 0.191 | 0.191 |
| Request to frame start | 15 | 0.140 | 0.460 | 0.460 |
| Frame start to submission | 15 | 1.826 | 4.473 | 4.473 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
