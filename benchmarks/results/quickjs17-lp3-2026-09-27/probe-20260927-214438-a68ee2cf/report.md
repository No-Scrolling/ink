# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11223 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.297 | 0.595 | 0.595 |
| React and transport | 15 | 11.205 | 24.306 | 24.306 |
| Json decode | 15 | 0.416 | 0.800 | 0.800 |
| Decode to apply | 15 | 0.058 | 0.107 | 0.107 |
| Apply and layout | 15 | 2.657 | 4.946 | 4.946 |
| Commit to frame submission | 15 | 2.947 | 5.267 | 5.267 |
| Input to frame submission | 15 | 17.064 | 34.674 | 34.674 |
| Commit to request | 15 | 0.105 | 0.355 | 0.355 |
| Request to frame start | 15 | 0.181 | 0.379 | 0.379 |
| Frame start to submission | 15 | 2.654 | 4.533 | 4.533 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
