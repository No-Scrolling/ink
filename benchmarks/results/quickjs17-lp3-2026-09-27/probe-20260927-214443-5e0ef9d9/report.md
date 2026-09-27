# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.314 | 0.667 | 0.667 |
| React and transport | 15 | 10.238 | 24.484 | 24.484 |
| Json decode | 15 | 0.317 | 0.782 | 0.782 |
| Decode to apply | 15 | 0.044 | 0.087 | 0.087 |
| Apply and layout | 15 | 1.703 | 4.098 | 4.098 |
| Commit to frame submission | 15 | 1.987 | 4.364 | 4.364 |
| Input to frame submission | 15 | 14.520 | 32.370 | 32.370 |
| Commit to request | 15 | 0.079 | 0.151 | 0.151 |
| Request to frame start | 15 | 0.150 | 0.282 | 0.282 |
| Frame start to submission | 15 | 1.741 | 3.931 | 3.931 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
