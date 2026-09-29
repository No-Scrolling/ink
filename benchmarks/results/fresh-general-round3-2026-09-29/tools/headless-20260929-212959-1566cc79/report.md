# Headless React-to-scene benchmark

CPU only. Native scrolling includes window materialisation and layout; no JS window event. Data updates replace all 1000 records in JavaScript. This is separate from the all-500-cells resize workload.

Measured updates: 20; discarded warm-up updates: 3.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 1.136 | 1.120 | 1.213 |
| inputToScene | 2.510 | 2.497 | 2.624 |
| javascriptAndTransport | 1.373 | 1.368 | 1.421 |
| nativeScroll | 0.057 | 0.056 | 0.062 |

Checks: native-only scroll, bounded visible scene, row action key, prepend, reverse anchor, keyed deletion, controlled Unicode input survives recycling, monotonic input acknowledgements, all mounted labels updated.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-212959-1566cc79/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-212959-1566cc79/assets --iterations 20 --warmup 3 --native-controls
```

