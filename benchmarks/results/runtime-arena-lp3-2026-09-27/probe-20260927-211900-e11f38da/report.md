# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11223 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.316 | 0.534 | 0.534 |
| React and transport | 15 | 12.640 | 26.597 | 26.597 |
| Json decode | 15 | 0.244 | 0.622 | 0.622 |
| Decode to apply | 15 | 0.039 | 0.080 | 0.080 |
| Apply and layout | 15 | 1.688 | 3.393 | 3.393 |
| Commit to frame submission | 15 | 1.836 | 4.093 | 4.093 |
| Input to frame submission | 15 | 16.855 | 34.425 | 34.425 |
| Commit to request | 15 | 0.070 | 0.187 | 0.187 |
| Request to frame start | 15 | 0.124 | 0.265 | 0.265 |
| Frame start to submission | 15 | 1.533 | 3.691 | 3.691 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
