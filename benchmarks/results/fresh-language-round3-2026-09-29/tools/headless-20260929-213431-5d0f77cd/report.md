# Headless React-to-scene benchmark

CPU scene only. Native and React modes share text/stack geometry; Row uses the standard composite geometry. React mode includes window messages and commits. No GPU or physical display timing.

Measured updates: 50; discarded warm-up updates: 5.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| NativeEdit | 0.961 | 0.857 | 1.365 |
| NativeScroll | 0.072 | 0.068 | 0.085 |
| NativeUnrelated | 0.222 | 0.218 | 0.263 |
| ReactScroll | 0.542 | 0.535 | 0.611 |
| RichScroll | 0.618 | 0.598 | 0.697 |
| RowEdit | 0.979 | 0.878 | 1.507 |
| RowScroll | 0.059 | 0.057 | 0.068 |
| RowUnrelated | 0.211 | 0.203 | 0.252 |

Checks: Text/Stack compiled to native, Row compiled to native, custom React state preserved, bounded windows, visible rows loaded, row callbacks, prepend, native reverse anchor, single row edits, fresh callback state, deletion, zero records for unrelated updates; one record per edit, native reorder list scroll and actions, native conversation initial end and scrolling.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-213431-5d0f77cd/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-213431-5d0f77cd/assets --iterations 50 --warmup 5 --list-compat
```

