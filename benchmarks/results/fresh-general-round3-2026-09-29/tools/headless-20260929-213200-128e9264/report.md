# Headless React-to-scene benchmark

CPU scene only. Native and React modes share text/stack geometry; Row uses the standard composite geometry. React mode includes window messages and commits. No GPU or physical display timing.

Measured updates: 20; discarded warm-up updates: 3.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| NativeEdit | 0.950 | 0.818 | 1.340 |
| NativeScroll | 0.073 | 0.069 | 0.074 |
| NativeUnrelated | 0.205 | 0.197 | 0.247 |
| ReactScroll | 0.523 | 0.513 | 0.594 |
| RichScroll | 0.565 | 0.570 | 0.583 |
| RowEdit | 0.966 | 0.843 | 1.426 |
| RowScroll | 0.058 | 0.058 | 0.061 |
| RowUnrelated | 0.200 | 0.196 | 0.219 |

Checks: Text/Stack compiled to native, Row compiled to native, custom React state preserved, bounded windows, visible rows loaded, row callbacks, prepend, native reverse anchor, single row edits, fresh callback state, deletion, zero records for unrelated updates; one record per edit, native reorder list scroll and actions, native conversation initial end and scrolling.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-213200-128e9264/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-213200-128e9264/assets --iterations 20 --warmup 3 --list-compat
```

