# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11223 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.316 | 0.585 | 0.585 |
| React and transport | 15 | 11.098 | 24.964 | 24.964 |
| Json decode | 15 | 0.276 | 0.661 | 0.661 |
| Decode to apply | 15 | 0.046 | 0.139 | 0.139 |
| Apply and layout | 15 | 1.973 | 5.009 | 5.009 |
| Commit to frame submission | 15 | 2.275 | 4.971 | 4.971 |
| Input to frame submission | 15 | 15.917 | 33.687 | 33.687 |
| Commit to request | 15 | 0.080 | 0.195 | 0.195 |
| Request to frame start | 15 | 0.180 | 0.335 | 0.335 |
| Frame start to submission | 15 | 2.047 | 4.466 | 4.466 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
