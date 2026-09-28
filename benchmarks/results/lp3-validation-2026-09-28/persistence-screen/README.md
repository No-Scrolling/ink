# React persistence screen

Question: does React's supported persistence mode reduce the cost of the unchanged 500-cell + resize workload?

This temporary renderer cloned host instances during reconciliation, then synchronised the committed snapshot with the existing native mutation protocol. The committed mirror kept event callbacks separate from uncommitted clones. Structural differences used the existing insert/remove operations; text updates retained the flat text batch. Only a committed container replacement published operations. No React source or benchmark workload changed.

The prototype passed all existing headless checks: every label and row gap on each update, thumb/content dragging, reverse, hide/show and count changes. It did not undergo full template or concurrent-render compatibility validation because the performance screen rejected it.

| Mac input-to-scene | Baseline | Prototype |
| --- | ---: | ---: |
| Samples | 1,200 | 1,200 |
| Mean | 1.649 ms | 2.212 ms |
| p95 | 1.958 ms | 2.589 ms |

ABBA order repeated three times, six processes per variant, 200 measured updates after 20 warm-ups. No build ran during measurement and no samples were discarded. The prototype mean was 34.2% slower. It adds host-instance cloning and snapshot synchronisation; those are plausible sources of additional work, not independently timed attributions. This rejects this implementation, not every possible persistent renderer.

Baseline `headless-20260928-123429-45152eb5`; prototype `headless-20260928-124528-6a2118de`. Raw measurements and the exact rejected patch are retained here. The source renderer was restored byte for byte to the pre-experiment copy. No persistence implementation is retained in production.
