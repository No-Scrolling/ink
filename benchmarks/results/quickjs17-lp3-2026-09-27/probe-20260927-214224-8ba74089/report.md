# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.306 | 0.685 | 0.685 |
| React and transport | 15 | 10.751 | 27.588 | 27.588 |
| Json decode | 15 | 0.356 | 0.683 | 0.683 |
| Decode to apply | 15 | 0.046 | 0.091 | 0.091 |
| Apply and layout | 15 | 1.866 | 4.050 | 4.050 |
| Commit to frame submission | 15 | 2.080 | 4.750 | 4.750 |
| Input to frame submission | 15 | 15.294 | 36.325 | 36.325 |
| Commit to request | 15 | 0.085 | 0.190 | 0.190 |
| Request to frame start | 15 | 0.148 | 0.349 | 0.349 |
| Frame start to submission | 15 | 1.841 | 4.210 | 4.210 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
