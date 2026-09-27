# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.316 | 0.654 | 0.654 |
| React and transport | 15 | 9.963 | 23.466 | 23.466 |
| Json decode | 15 | 0.274 | 0.715 | 0.715 |
| Decode to apply | 15 | 0.046 | 0.083 | 0.083 |
| Apply and layout | 15 | 1.735 | 3.882 | 3.882 |
| Commit to frame submission | 15 | 2.050 | 4.037 | 4.037 |
| Input to frame submission | 15 | 14.277 | 31.417 | 31.417 |
| Commit to request | 15 | 0.083 | 0.148 | 0.148 |
| Request to frame start | 15 | 0.145 | 0.296 | 0.296 |
| Frame start to submission | 15 | 1.821 | 3.593 | 3.593 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
