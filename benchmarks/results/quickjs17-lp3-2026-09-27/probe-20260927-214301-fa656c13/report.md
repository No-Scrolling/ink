# React to native update

Accepted updates: 15; rejected: 0.
Median commit payload: 11224 bytes.

| Stage | Samples | Median (ms) | p95 (ms) | Maximum (ms) |
| --- | ---: | ---: | ---: | ---: |
| Input to dispatch | 15 | 0.276 | 0.498 | 0.498 |
| React and transport | 15 | 11.145 | 24.775 | 24.775 |
| Json decode | 15 | 0.362 | 0.682 | 0.682 |
| Decode to apply | 15 | 0.056 | 0.100 | 0.100 |
| Apply and layout | 15 | 2.082 | 4.164 | 4.164 |
| Commit to frame submission | 15 | 2.509 | 4.472 | 4.472 |
| Input to frame submission | 15 | 16.822 | 32.848 | 32.848 |
| Commit to request | 15 | 0.095 | 0.162 | 0.162 |
| Request to frame start | 15 | 0.186 | 0.307 | 0.307 |
| Frame start to submission | 15 | 2.245 | 4.015 | 4.015 |

Isolated counter or update-fixture interactions only. React and transport combines JS execution, JSON encoding and queue waits. Driver presentation timestamps are reported separately when available; delayed feedback can leave the final frames unmeasured. Neither submission nor driver timing measures the physical panel. Instrumentation adds overhead.

Raw timestamps and samples: [result.json](result.json).
