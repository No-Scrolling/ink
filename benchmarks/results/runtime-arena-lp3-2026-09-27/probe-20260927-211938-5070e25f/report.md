# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11223 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.297 | 0.576 | 0.576 |
| React and transport | 15 | 10.182 | 24.024 | 24.024 |
| Json decode | 15 | 0.285 | 0.717 | 0.717 |
| Decode to apply | 15 | 0.054 | 0.203 | 0.203 |
| Apply and layout | 15 | 1.965 | 3.877 | 3.877 |
| Commit to frame submission | 15 | 2.377 | 4.249 | 4.249 |
| Input to frame submission | 15 | 15.526 | 32.908 | 32.908 |
| Commit to request | 15 | 0.098 | 0.138 | 0.138 |
| Request to frame start | 15 | 0.163 | 0.336 | 0.336 |
| Frame start to submission | 15 | 2.133 | 3.870 | 3.870 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
