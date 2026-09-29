# Headless React-to-scene benchmark

CPU scene only. Native and React modes share text/stack geometry; Row uses the standard composite geometry. React mode includes window messages and commits. No GPU or physical display timing.

Measured updates: 20; discarded warm-up updates: 3.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| NativeEdit | 1.010 | 0.896 | 1.486 |
| NativeScroll | 0.067 | 0.067 | 0.070 |
| NativeUnrelated | 0.233 | 0.222 | 0.312 |
| ReactScroll | 0.599 | 0.590 | 0.684 |
| RichScroll | 0.674 | 0.635 | 0.826 |
| RowEdit | 1.172 | 1.023 | 1.585 |
| RowScroll | 0.081 | 0.061 | 0.153 |
| RowUnrelated | 0.280 | 0.270 | 0.346 |

Checks: Text/Stack compiled to native, Row compiled to native, custom React state preserved, bounded windows, visible rows loaded, row callbacks, prepend, native reverse anchor, single row edits, fresh callback state, deletion, zero records for unrelated updates; one record per edit, native reorder list scroll and actions, native conversation initial end and scrolling.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-213358-cccdb9a8/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-213358-cccdb9a8/assets --iterations 20 --warmup 3 --list-compat
```

