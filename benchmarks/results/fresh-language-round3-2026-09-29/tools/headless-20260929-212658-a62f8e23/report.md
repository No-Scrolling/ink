# Headless React-to-scene benchmark

CPU only. Native scrolling includes window materialisation and layout; no JS window event. Data updates replace all 1000 records in JavaScript. This is separate from the all-500-cells resize workload.

Measured updates: 100; discarded warm-up updates: 10.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 0.997 | 0.981 | 1.089 |
| inputToScene | 2.266 | 2.252 | 2.403 |
| javascriptAndTransport | 1.269 | 1.264 | 1.330 |
| nativeScroll | 0.054 | 0.054 | 0.060 |

Checks: native-only scroll, bounded visible scene, row action key, prepend, reverse anchor, keyed deletion, controlled Unicode input survives recycling, monotonic input acknowledgements, all mounted labels updated.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212658-a62f8e23/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212658-a62f8e23/assets --iterations 100 --warmup 10 --native-controls
```

