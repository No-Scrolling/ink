# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.297 | 0.535 | 0.535 |
| React and transport | 15 | 10.193 | 25.340 | 25.340 |
| Json decode | 15 | 0.312 | 0.708 | 0.708 |
| Decode to apply | 15 | 0.047 | 0.112 | 0.112 |
| Apply and layout | 15 | 1.683 | 4.373 | 4.373 |
| Commit to frame submission | 15 | 2.003 | 5.464 | 5.464 |
| Input to frame submission | 15 | 14.425 | 33.884 | 33.884 |
| Commit to request | 15 | 0.080 | 0.181 | 0.181 |
| Request to frame start | 15 | 0.143 | 0.414 | 0.414 |
| Frame start to submission | 15 | 1.774 | 4.990 | 4.990 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
