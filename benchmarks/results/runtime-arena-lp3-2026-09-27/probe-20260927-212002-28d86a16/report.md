# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.316 | 0.462 | 0.462 |
| React and transport | 15 | 12.915 | 28.507 | 28.507 |
| Json decode | 15 | 0.269 | 0.684 | 0.684 |
| Decode to apply | 15 | 0.039 | 0.102 | 0.102 |
| Apply and layout | 15 | 1.659 | 4.639 | 4.639 |
| Commit to frame submission | 15 | 1.960 | 5.047 | 5.047 |
| Input to frame submission | 15 | 17.038 | 39.299 | 39.299 |
| Commit to request | 15 | 0.085 | 0.186 | 0.186 |
| Request to frame start | 15 | 0.125 | 0.297 | 0.297 |
| Frame start to submission | 15 | 1.705 | 4.565 | 4.565 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
