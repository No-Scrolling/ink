# Compiled list projection reuse — 28 September 2026

Existing public List API; 1,000 records on the development Mac. Three alternating baseline/candidate pairs, each with 5 warm-ups and 50 measured iterations per workload. Values below are means of the three run means; [raw results](paired.json) retain complete checks and timings.

| CPU input → scene | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| Text/Stack unrelated parent update | 2.579 ms | 1.129 ms | 56% |
| Text/Stack one-row edit | 2.953 ms | 1.579 ms | 47% |
| Standard Row unrelated parent update | 6.223 ms | 1.022 ms | 84% |
| Standard Row one-row edit | 6.732 ms | 1.575 ms | 77% |
| Text/Stack native scroll | 0.069 ms | 0.070 ms | Approximately unchanged |
| Standard Row native scroll | 0.087 ms | 0.085 ms | Approximately unchanged |
| Full suite | 1,179 ms | 421 ms | 64% |

Baseline: `headless-20260928-173651-39dbacd1`. Candidate: `headless-20260928-173802-a76b9b33`. Candidate Hyperfine mean: 0.427 seconds, standard deviation 0.0012 seconds over five process runs. Prepared binaries/assets are retained in their `.agent-tools` directories. Paired runs use identical fixture/harness sources. `headless-20260928-173734-39e8496e` failed the source-change guard during development and is not measurement evidence.

## Change

The compiler emits a small visual-dependency reader alongside each eligible row declaration. The adapter compares its values against the last committed record and skips constructing and normalising unchanged native row data. Each declaration includes a stable site identity so distinct render expressions with the same native template cannot share an incompatible projection cache. Measurement keys still invalidate the native record independently.

Inline arrow handlers need no visual dependency: they always represent an enabled action. On an event, the adapter projects only that row using the latest committed render's plan and current item/index, then invokes the handler. External function-valued dependencies still compare by identity. Fixed object literals track their values; arbitrary object-valued dependencies always reproject. This preserves conservative behaviour for mutable objects. Unsupported JSX continues through React.

There is still an O(N) key/dependency scan and small per-row bookkeeping allocation when the parent invalidates the list. This change removes full projection and recursive normalisation for unchanged rows, not all JavaScript list work. Rust remains responsible for windows, recycling and applying changed records.

## Validation

- All existing compiled-list and React-fallback headless checks pass, including edits, zero/one transferred records, prepend, reorder anchoring, deletion and stateful React rows.
- Extended the previously authorised stale-callback check to tap an unchanged row after repeated parent updates. Final run `headless-20260928-173829-e475ad37` passes this additional assertion as well as the edited-row assertion.
- All 12 existing compiler checks pass; Ink and Reverb TypeScript checks pass. Reverb's development JavaScript bundle compiles successfully.
- No emulator or LP3 run for this change. These are Mac CPU measurements; they exclude Android scheduling, GPU rendering and display latency.
