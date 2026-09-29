# Headless React-to-scene benchmark

CPU scene only. Native and React modes share text/stack geometry; Row uses the standard composite geometry. React mode includes window messages and commits. No GPU or physical display timing.

Measured updates: 20; discarded warm-up updates: 3.

| Stage | Mean (ms) | Median (ms) | p95 (ms) |
| --- | ---: | ---: | ---: |
| NativeEdit | 0.981 | 0.873 | 1.430 |
| NativeScroll | 0.066 | 0.066 | 0.075 |
| NativeUnrelated | 0.214 | 0.201 | 0.276 |
| ReactScroll | 0.505 | 0.504 | 0.542 |
| RichScroll | 0.550 | 0.549 | 0.598 |
| RowEdit | 0.955 | 0.852 | 1.325 |
| RowScroll | 0.055 | 0.055 | 0.061 |
| RowUnrelated | 0.193 | 0.193 | 0.202 |

Checks: Text/Stack compiled to native, Row compiled to native, custom React state preserved, bounded windows, visible rows loaded, row callbacks, prepend, native reverse anchor, single row edits, fresh callback state, deletion, zero records for unrelated updates; one record per edit, native reorder list scroll and actions, native conversation initial end and scrolling.

Run the prepared binary again:

```sh
/Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-212711-ccda1b28/ink-headless --assets /Users/vandam/Developer/ink/benchmarks/results/fresh-general-round3-2026-09-29/tools/headless-20260929-212711-ccda1b28/assets --iterations 20 --warmup 3 --list-compat
```

