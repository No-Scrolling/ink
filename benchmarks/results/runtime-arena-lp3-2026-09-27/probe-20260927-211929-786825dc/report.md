# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.302 | 0.404 | 0.404 |
| React and transport | 15 | 10.695 | 25.641 | 25.641 |
| Json decode | 15 | 0.326 | 0.795 | 0.795 |
| Decode to apply | 15 | 0.043 | 0.101 | 0.101 |
| Apply and layout | 15 | 1.893 | 4.364 | 4.364 |
| Commit to frame submission | 15 | 2.091 | 4.346 | 4.346 |
| Input to frame submission | 15 | 15.109 | 34.122 | 34.122 |
| Commit to request | 15 | 0.083 | 0.251 | 0.251 |
| Request to frame start | 15 | 0.165 | 0.287 | 0.287 |
| Frame start to submission | 15 | 1.848 | 3.834 | 3.834 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
