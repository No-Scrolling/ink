# Headless React-to-scene benchmark

CPU scene only. Native and React modes share text/stack geometry; Row uses the standard composite geometry. React mode includes window messages and commits. No GPU or physical display timing.

Measured updates: 50; discarded warm-up updates: 5.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| NativeEdit | 1.044 | 0.935 | 1.576 |
| NativeScroll | 0.070 | 0.069 | 0.080 |
| NativeUnrelated | 0.274 | 0.248 | 0.436 |
| ReactScroll | 0.716 | 0.649 | 1.127 |
| RichScroll | 0.547 | 0.531 | 0.635 |
| RowEdit | 0.916 | 0.810 | 1.343 |
| RowScroll | 0.060 | 0.059 | 0.069 |
| RowUnrelated | 0.195 | 0.190 | 0.229 |

Checks: Text/Stack compiled to native, Row compiled to native, custom React state preserved, bounded windows, visible rows loaded, row callbacks, prepend, native reverse anchor, single row edits, fresh callback state, deletion, zero records for unrelated updates; one record per edit, native reorder list scroll and actions, native conversation initial end and scrolling.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212442-22e8026d/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-212442-22e8026d/assets --iterations 50 --warmup 5 --list-compat
```

