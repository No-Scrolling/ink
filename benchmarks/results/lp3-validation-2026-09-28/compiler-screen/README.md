# Physical LP3 compiler screen

The unchanged 500-cell + resize harness compared two additional Android C compiler settings with the retained baseline. ABBA order repeated three times, six processes and 600 measured updates per variant, with 20 warm-ups per process. All label, geometry, scrolling, reverse, hide/show and count-change checks passed. No builds ran during paired measurement. Neither option was retained.

| Setting | Baseline mean | Candidate mean | Baseline p95 | Candidate p95 |
| --- | ---: | ---: | ---: | ---: |
| `-fno-unroll-loops` | 7.200 ms | 7.249 ms | 7.593 ms | 7.599 ms |
| `-fomit-frame-pointer -momit-leaf-frame-pointer` | 7.202 ms | 7.142 ms | 7.545 ms | 7.502 ms |

Loop unrolling showed no improvement. Frame-pointer omission showed less than 1% difference, with the larger process-level variation visible in the raw results; this screen does not establish a useful gain. The earlier emulator rejection of Android loop-unrolling changes is therefore consistent with this physical-phone screen. There is no justification here to change the compiler configuration or debugging trade-offs.

Baseline: `headless-20260928-114535-e22ca8e5`. No-unroll candidate: `headless-20260928-123814-1bb99768`. Frame-pointer candidate: `headless-20260928-124032-b349b63e`. Each candidate has a distinct binary hash. The copied QuickJS source guard passed, and saved dependency build output confirms the requested flags reached the C build. All measurements run as Android shell and exclude Activity scheduling and frame submission; they do not satisfy the full-app p95 goal. The settings were supplied only in command environments. No project compiler configuration was changed.
