# Headless React-to-scene benchmark

CPU scene only. Native and React modes share text/stack geometry; Row uses the standard composite geometry. React mode includes window messages and commits. No GPU or physical display timing.

Measured updates: 50; discarded warm-up updates: 5.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| NativeEdit | 0.852 | 0.771 | 1.129 |
| NativeScroll | 0.066 | 0.065 | 0.073 |
| NativeUnrelated | 0.193 | 0.187 | 0.243 |
| ReactScroll | 0.469 | 0.467 | 0.506 |
| RichScroll | 0.529 | 0.517 | 0.558 |
| RowEdit | 0.839 | 0.764 | 1.177 |
| RowScroll | 0.051 | 0.050 | 0.059 |
| RowUnrelated | 0.183 | 0.181 | 0.199 |

Checks: Text/Stack compiled to native, Row compiled to native, custom React state preserved, bounded windows, visible rows loaded, row callbacks, prepend, native reverse anchor, single row edits, fresh callback state, deletion, zero records for unrelated updates; one record per edit, native reorder list scroll and actions, native conversation initial end and scrolling.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-214434-43f4c0e8/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools/headless-20260929-214434-43f4c0e8/assets --iterations 50 --warmup 5 --list-compat
```

