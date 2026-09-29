# Independent evidence index

The primary deliverable is `report.md`; `final-provenance.json` records the final source and diagnostic SHA-256 values. Only freshly generated outputs from this directory were read. No application source/repository tests were edited, no tests were added, and no device state was changed.

## Final semantic controls

- `proxy-final-quickjs.json`: actual renderer/AppRuntime release control, root/nested/captured/callable Proxy and ordinary-function accessor cases. All fallback cases match React's 32 visible reads/calls without additional reflection traps.
- `function-property-final-quickjs.json`: actual compiler/public List/release QuickJS, 16 cases. Accessors/custom cycles fall back; direct mutable function fields send 1,000 correct changed records; declared/arrow/bound callbacks remain native.
- `function-property-source.tsx`, `function-property-runtime.tsx`: original and actual transformed author code. `function-property-runtime-builder.ts` reproduces bundle preparation using local package resolution; it writes no application source.
- `compiled-capture-depth-final-release.json` and `compiled-capture-depth-final-ink-dev.json`: 28 actual-compiler/public-List cases each, no errors; normal/shared chains through 24 levels remain native and depth 64 falls back. Source/preparation is `compiler-depth-runtime-builder.ts` and `compiled-capture-depth-{source,runtime}.tsx`.
- `quickjs-driver/`: observational AppRuntime runner, not a repository test. It consumes native commits/events and stops after the tagged observation. It does not apply commits to a scene or perform GPU/pixel inspection. `ink-dev` copies the repository's profile settings.

## Historical controls, deliberately retained

- `projection-diagnostic.json`: initial Bun observation, before final native eligibility changes; Bun does not install the native Proxy helper. This is not final QuickJS evidence.
- `function-property-initial-{source,runtime}.tsx`, `function-property-initial-quickjs.json`: the demonstrated ordinary-function property/accessor defect before its final fix.
- `capture-depth-quickjs.json`, `capture-depth-quickjs-release.json`, `capture-depth-quickjs-ink-dev.json`: manual compiler-interface controls establishing a default unoptimised-dev failure and falsifying the app-profile claim.
- `compiled-capture-depth-{dev,release,ink-dev}.json`: actual-compiler controls of the same profile distinction before the final function-own-field change.
- `shared-capture-first-failure.*`: initial exploratory default-dev failure; later controlled profile results determine classification.
- `compiled-capture-depth-*-before.log`: preparation failed because an already-written Bun output was accidentally rewritten to an empty file; the runner timed out. The builder was corrected before any claimed actual-compiler results. These logs establish no application defect.

## Existing checks and CPU fixtures

- `final-typecheck.log`, `final-compiler-existing-tests.log`, `final-cargo-existing-tests.log`: existing checks only. Core/runtime execute 6/2 tests; audio/protocol execute no unit cases in this invocation.
- `tools/headless-20260929-212442-22e8026d`: initial public List, 50 updates/5 warm-ups.
- `tools/headless-20260929-212559-960f4f67`: ordinary React 500-cell update/resize, 100/10.
- `tools/headless-20260929-212640-f07e1bb9`: native binding 500-cell update/resize, 100/10.
- `tools/headless-20260929-212658-a62f8e23`: native controls/full 1,000-record replacement, 100/10.
- `tools/headless-20260929-212754-4d684b42`: coarse React phase diagnostic, 100/10; nested timings and instrumentation overhead are explicit.
- `tools/headless-20260929-213317-29ee3e59` and `tools/headless-20260929-213431-5d0f77cd`: later list controls around shared capture changes, 50/5.
- `tools/headless-20260929-214434-43f4c0e8`: final function-property functional fix, 50/5, all checks pass. One final cosmetic local-variable rename changes the native-list input hash; all 94 other inputs match. Final semantic bundles were rebuilt after that rename.

Each tools result retains exact input/dependency hashes and results. Prepared binaries/assets and diagnostic bundles were removed after the audit. Rebuild them before replaying saved commands; hashes continue to describe the original artefacts, and rebuilding with current source does not recreate an earlier source snapshot. Diagnostic source and builders remain. These are Mac CPU/scene observations, excluding Android input/lifecycle/GPU/presentation and physical-panel response. No quiet-system or small-speedup claim is made.

Use `INK_AGENT_TOOLS_DIR=/Users/vandam/Developer/ink/benchmarks/results/fresh-language-round3-2026-09-29/tools scripts/agent-tools result ID --full` to inspect these fresh results. Recreate the final source inventory with `uv run --script benchmarks/results/fresh-language-round3-2026-09-29/final-provenance.py`.
