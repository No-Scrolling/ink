# Headless React-to-scene benchmark

CPU only. Native scrolling includes window materialisation and layout; no JS window event. Data updates replace all 1000 records in JavaScript. This is separate from the all-500-cells resize workload.

Measured updates: 20; discarded warm-up updates: 3.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| applyAndLayout | 1.193 | 1.162 | 1.267 |
| inputToScene | 2.593 | 2.559 | 2.847 |
| javascriptAndTransport | 1.399 | 1.385 | 1.465 |
| nativeScroll | 0.058 | 0.058 | 0.061 |

Checks: native-only scroll, bounded visible scene, row action key, prepend, reverse anchor, keyed deletion, controlled Unicode input survives recycling, monotonic input acknowledgements, all mounted labels updated.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-212344-0103308a/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-212344-0103308a/assets --iterations 20 --warmup 3 --native-controls
```

