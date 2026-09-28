# QuickJS allocation screens

Neither prototype is selected in the production Cargo manifest. Both were temporary local QuickJS changes, tested with the unchanged React fixture on Mac in six paired processes per variant, 200 measured updates after 20 warm-ups.

| Prototype | Baseline mean / p95 | Candidate mean / p95 | Decision |
| --- | ---: | ---: | --- |
| Cache decimal strings for integers 0–1023 | 1.750 / 2.000 ms | 1.737 / 2.022 ms | Rejected: less than 1% mean difference, worse p95 |
| Initial object property capacity 8 rather than 2 | 1.607 / 1.750 ms | 1.561 / 1.694 ms | Deferred: modest Mac gain, no phone or memory validation |

Integer-string candidate: `headless-20260928-125022-d5f4b605`. Object-capacity candidate: `headless-20260928-125258-e7af4809`. Both controls use `headless-20260928-124758-52d0cae4`. Patches and paired raw results are preserved beside this report. Existing scene checks passed, but neither experiment received comprehensive interpreter semantic testing. The object-capacity experiment could increase memory consumption for small objects and arrays. No production gain is claimed for either.
