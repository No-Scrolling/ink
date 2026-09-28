# QuickJS property-access investigation

The retained engine is unchanged. Two local QuickJS prototypes were rejected after paired Mac and LP3 headless comparisons. All measured runs retained the unchanged 500-cell + resize fixture and passed its label, geometry and scrolling checks.

## Native sampling

Android diagnostic build `headless-20260928-115224-4a790ef1` retained debug symbols. Simpleperf recorded 3,016 samples over approximately 6.04 CPU seconds, including startup, warm-up, 800 updates and compatibility checks. This profile locates costs; it is not a release timing comparison. The raw recording remains at `.agent-tools/interpreter-profile/perf.data`.

| Sampled location | Share of total CPU time | Interpretation |
| --- | ---: | --- |
| `JS_CallInternal` | 56.63% self | Interpreter execution, including many different bytecodes |
| `get_field` | approximately 21.25% | Ordinary property reads, attributed from sampled interpreter source locations |
| `put_field` | approximately 2.65% | Property writes |
| `js_arena_malloc` | 3.25% self | Allocation |
| `JS_GetPropertyInternal` | 2.19% self | General property-access helper |
| Native tree apply callback | 8.16% inclusive | Includes callees; overlaps other rows |

Rows are overlapping views and must not be added. Source-line attribution is approximate, especially around inlined helpers. About 16.41% of all samples had unresolved source lines inside the interpreter. The shared `js_array_every` implementation also handles operations such as map/filter, so its inclusive samples do not establish an expensive application `.every()` call. Full symbol and source attribution are in `report.txt` and `opcode-samples.json`.

## Build-cache correction

The initial fusion runs (`headless-20260928-121145-bda7818c` and `headless-20260928-121617-1a10480a`) accidentally reused the earlier property-cache binaries. Their SHA-256 hashes are identical on each platform, as recorded in `stale-build-audit.json`. The dependency build script did not watch its C source directory. Those timings do **not** test fusion; the original compressed results remain for audit and are marked invalid in `paired-summary.json`. The temporary fork now watches its sources. The headless tool also records resolved QuickJS source hashes and verifies that the C/header copies in Cargo’s build output match before running. A deliberate mismatch was rejected.

Corrected fusion builds are `headless-20260928-122806-d02dc0c3` (Mac) and `headless-20260928-123151-70cc91e2` (Android). The table below uses the corrected paired results, retained in `verified-fusion-*-paired.json.gz` and `verified-fusion-summary.json`.

## Paired prototype results

Each comparison used ABBA order repeated three times, six processes per variant. Mac processes measured 200 updates after 20 warm-ups; LP3 processes measured 100 after 20. Values below pool every measured update, without discarding slow samples. The phone runs use the Android shell CPU harness, not Activity frame submission.

| Prototype / platform | Baseline mean | Prototype mean | Baseline p95 | Prototype p95 |
| --- | ---: | ---: | ---: | ---: |
| Property-slot cache / Mac | 1.706 ms | 1.662 ms | 1.899 ms | 1.908 ms |
| Property-slot cache / LP3 | 7.137 ms | 7.156 ms | 7.593 ms | 7.544 ms |
| Local/argument property-read fusion / Mac | 1.526 ms | 1.510 ms | 1.653 ms | 1.646 ms |
| Local/argument property-read fusion / LP3 | 7.204 ms | 7.166 ms | 7.568 ms | 7.574 ms |

The slot cache saved the last observed property index, validating the current object shape before using it. It did not improve phone mean duration. The fusion prototype combined loading a local or argument with reading its property, avoiding a temporary receiver reference increment/decrement. Its corrected phone comparison showed only a 0.52% mean difference and slightly worse p95. Neither demonstrated a dependable phone improvement sufficient to justify maintaining a custom interpreter and bytecode format.

Both patches are preserved here for investigation, not as supported implementations. They passed this fixture but did not undergo comprehensive JavaScript semantic validation. The initial Mac cache screen preceded an additional null-atom guard used for its Android screen. These first cache builds were distinct from their baselines; the later source edits were the ones affected by caching. Source snapshots and prepared binaries remain in the operation directories; the ignored fork was not included in the ordinary tool's source hashes. These patch files are therefore part of the prototype provenance.

Builds: baseline Mac `headless-20260928-114638-86307b7e`, baseline Android `headless-20260928-114535-e22ca8e5`; cache Mac `headless-20260928-120046-3bd843e3`, cache Android `headless-20260928-120323-743b9de6`; corrected fusion Mac `headless-20260928-122806-d02dc0c3`, corrected fusion Android `headless-20260928-123151-70cc91e2`. Compressed paired files retain raw samples and `paired-summary.json` includes individual process means.

The root Cargo manifest and lockfile were restored exactly to their pre-experiment copies. The physical app p95 gate remains unmet. The property census below narrows the next investigation towards React traversal itself; these measurements do not demonstrate that another interpreter fork would help.


## Property-read census

A separate diagnostic counted `get_field` operations by property name. Subtracting a checked 100-update process from a checked 200-update process, both after 20 warm-ups, gives approximately **65,560.5 reads per extra update**. Both processes use the unchanged fixture and include the same startup and compatibility checks. The half read reflects state-dependent control flow, not fractional instructions. Counts are not timings and exclude other property-access bytecodes.

| Property | Reads per update |
| --- | ---: |
| alternate | 5,776 |
| flags | 4,768 |
| child | 4,223 |
| sibling | 4,050 |
| type | 4,008 |
| memoizedProps | 3,955 |
| tag | 2,973 |
| pendingProps | 2,882 |
| ref | 2,807 |
| subtreeFlags | 2,324 |

React node traversal and state dominate these counts. Each of Ink's `size`, `width`, `align`, `maxLines` and `tabularNumbers` appears about 1,002 times. Even eliminating all those checks would remove only a fraction of total reads, and would be incorrect when styling changes. This rules out treating the small Text comparison as the main remaining bottleneck.

The counter patch and all counts are retained in `property-diagnostic.patch`, `property-per-update.json` and `property-counts.json.gz`. It pins counted atoms until runtime shutdown and has substantial diagnostic overhead; its durations are not speed measurements. The instrumented binary is under `headless-20260928-122638-12eae4dc`. That tool operation failed JSON parsing because the diagnostic writes to stderr; subsequent direct invocations captured stdout/stderr separately and completed all scene checks. No counter code is selected by the restored production dependency.
