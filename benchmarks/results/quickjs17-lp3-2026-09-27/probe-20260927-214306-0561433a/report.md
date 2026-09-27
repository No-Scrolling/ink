# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.304 | 0.609 | 0.609 |
| React and transport | 15 | 10.859 | 25.311 | 25.311 |
| Json decode | 15 | 0.310 | 0.703 | 0.703 |
| Decode to apply | 15 | 0.044 | 0.094 | 0.094 |
| Apply and layout | 15 | 1.809 | 4.035 | 4.035 |
| Commit to frame submission | 15 | 2.225 | 4.228 | 4.228 |
| Input to frame submission | 15 | 15.362 | 32.489 | 32.489 |
| Commit to request | 15 | 0.089 | 0.167 | 0.167 |
| Request to frame start | 15 | 0.166 | 0.256 | 0.256 |
| Frame start to submission | 15 | 1.982 | 3.828 | 3.828 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
