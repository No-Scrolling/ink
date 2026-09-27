# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.288 | 0.730 | 0.730 |
| React and transport | 15 | 10.322 | 24.785 | 24.785 |
| Json decode | 15 | 0.267 | 0.679 | 0.679 |
| Decode to apply | 15 | 0.046 | 0.084 | 0.084 |
| Apply and layout | 15 | 1.657 | 4.186 | 4.186 |
| Commit to frame submission | 15 | 2.053 | 3.817 | 3.817 |
| Input to frame submission | 15 | 14.412 | 32.165 | 32.165 |
| Commit to request | 15 | 0.077 | 0.158 | 0.158 |
| Request to frame start | 15 | 0.142 | 0.252 | 0.252 |
| Frame start to submission | 15 | 1.796 | 3.440 | 3.440 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
