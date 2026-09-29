# Headless React-to-scene benchmark

CPU scene only. Native and React modes share text/stack geometry; Row uses the standard composite geometry. React mode includes window messages and commits. No GPU or physical display timing.

Measured updates: 20; discarded warm-up updates: 3.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| NativeEdit | 0.852 | 0.751 | 1.130 |
| NativeScroll | 0.066 | 0.066 | 0.073 |
| NativeUnrelated | 0.186 | 0.183 | 0.209 |
| ReactScroll | 0.470 | 0.467 | 0.498 |
| RichScroll | 0.548 | 0.546 | 0.589 |
| RowEdit | 0.908 | 0.823 | 1.261 |
| RowScroll | 0.052 | 0.051 | 0.060 |
| RowUnrelated | 0.191 | 0.188 | 0.222 |

Checks: Text/Stack compiled to native, Row compiled to native, custom React state preserved, bounded windows, visible rows loaded, row callbacks, prepend, native reverse anchor, single row edits, fresh callback state, deletion, zero records for unrelated updates; one record per edit, native reorder list scroll and actions, native conversation initial end and scrolling.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-214432-6c8220ea/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-214432-6c8220ea/assets --iterations 20 --warmup 3 --list-compat
```

