# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.321 | 0.698 | 0.698 |
| React and transport | 15 | 25.527 | 27.264 | 27.264 |
| Json decode | 15 | 0.433 | 0.700 | 0.700 |
| Decode to apply | 15 | 0.069 | 0.092 | 0.092 |
| Apply and layout | 15 | 2.868 | 4.330 | 4.330 |
| Commit to frame submission | 15 | 3.388 | 4.146 | 4.146 |
| Input to frame submission | 15 | 32.710 | 35.985 | 35.985 |
| Commit to request | 15 | 0.127 | 0.247 | 0.247 |
| Request to frame start | 15 | 0.222 | 0.489 | 0.489 |
| Frame start to submission | 15 | 3.030 | 3.476 | 3.476 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
