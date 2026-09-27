# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.343 | 0.658 | 0.658 |
| React and transport | 15 | 12.943 | 28.714 | 28.714 |
| Json decode | 15 | 0.265 | 0.543 | 0.543 |
| Decode to apply | 15 | 0.041 | 0.074 | 0.074 |
| Apply and layout | 15 | 1.521 | 3.230 | 3.230 |
| Commit to frame submission | 15 | 2.116 | 4.172 | 4.172 |
| Input to frame submission | 15 | 17.787 | 36.058 | 36.058 |
| Commit to request | 15 | 0.072 | 0.205 | 0.205 |
| Request to frame start | 15 | 0.126 | 0.367 | 0.367 |
| Frame start to submission | 15 | 1.899 | 3.671 | 3.671 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
