# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.297 | 0.558 | 0.558 |
| React and transport | 15 | 10.288 | 24.067 | 24.067 |
| Json decode | 15 | 0.265 | 0.691 | 0.691 |
| Decode to apply | 15 | 0.044 | 0.099 | 0.099 |
| Apply and layout | 15 | 1.714 | 4.228 | 4.228 |
| Commit to frame submission | 15 | 2.031 | 4.423 | 4.423 |
| Input to frame submission | 15 | 14.715 | 33.694 | 33.694 |
| Commit to request | 15 | 0.083 | 0.170 | 0.170 |
| Request to frame start | 15 | 0.141 | 0.284 | 0.284 |
| Frame start to submission | 15 | 1.817 | 4.001 | 4.001 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
