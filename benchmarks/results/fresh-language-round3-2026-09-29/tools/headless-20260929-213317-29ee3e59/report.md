# Headless React-to-scene benchmark

CPU scene only. Native and React modes share text/stack geometry; Row uses the standard composite geometry. React mode includes window messages and commits. No GPU or physical display timing.

Measured updates: 50; discarded warm-up updates: 5.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| NativeEdit | 0.960 | 0.878 | 1.402 |
| NativeScroll | 0.067 | 0.067 | 0.072 |
| NativeUnrelated | 0.218 | 0.213 | 0.259 |
| ReactScroll | 0.578 | 0.575 | 0.630 |
| RichScroll | 0.648 | 0.632 | 0.719 |
| RowEdit | 1.108 | 0.943 | 1.645 |
| RowScroll | 0.058 | 0.057 | 0.060 |
| RowUnrelated | 0.246 | 0.241 | 0.321 |

Checks: Text/Stack compiled to native, Row compiled to native, custom React state preserved, bounded windows, visible rows loaded, row callbacks, prepend, native reverse anchor, single row edits, fresh callback state, deletion, zero records for unrelated updates; one record per edit, native reorder list scroll and actions, native conversation initial end and scrolling.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-213317-29ee3e59/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-213317-29ee3e59/assets --iterations 50 --warmup 5 --list-compat
```

