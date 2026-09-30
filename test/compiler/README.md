# Compiler contracts

Run `bun test test/compiler` from the repository root.

| Suite | Observable contract and realistic failure caught | Coverage limits |
| --- | --- | --- |
| `native-lists.test.ts` | Execute compiler output against actual Ink list helpers. Nested text/property bindings resolve to explicit expected values; indices and captures retain their values; actions remain deferred; Row fields retain presentation and long-press behaviour; several lists and occupied helper names do not collide; one React-only list does not suppress another eligible list. | Exercises `compileNativeLists` and the SDK helper protocol in Bun. Does not mount React, run QuickJS, render native rows or run the complete JavaScript bundler. |
| `native-lists-fallback.test.ts` | Unsupported components, executable expressions, refs, spreads, assignment, entities and conditional row shapes remain byte-for-byte unchanged. Shadowed component names retain React ownership. Aliased imports produce a working text projector. Linked Rows yield a deferred action that reaches the real navigation API. | Preserving source guards the React path; component lifetimes themselves require runtime integration. Navigation deliberately has no mounted router here. |
| `file-routes.test.ts` | Real filesystem traversal produces explicit route/layout ancestry, omits group names from URLs, ignores hidden/private/non-page files, retains unchanged output, and rejects duplicate, ambiguous, repeated-parameter, malformed-parameter and missing-home graphs before writing output. | Decodes generated route data through the TypeScript AST; does not mount `FileNavigator` or build an APK. |
| `icon-usage.test.ts` | Asset resolution reflects the largest known host size, retains default resolution for unknown/dynamic/spread uses, respects aliases and shadowed/type-only/unused imports, and handles SVGs and exports. | Tests analysis with declared icon references and an SVG reference resolver; rasterisation and bundled asset manifests remain outside this suite. |
| `capabilities.test.ts` | The real compiler CLI resolves explicit transitive capabilities, includes requirements from surviving package code, removes requirements from discarded code, rejects invalid declarations/configuration and TypeScript errors before publishing an application, and removes stale split-web assets when modes or imports change. | Temporary projects use actual workspace dependencies and bundling, with explicit expected manifests. Does not build Android variants or prove platform permission/source-group wiring. |

The CLI contracts use the `ink-dev` Rust profile and compile production JavaScript bundles. The runner's `--profile` option selects the Rust optimisation profile for native integration suites; it does not select a JavaScript development build. These tests remove their temporary project directories in `finally`.

A regression requires `text-input-full`, an unsupported capability, to produce a configuration diagnostic rather than a compiler panic. The split-web mode regression requires switching between separate and inline web bundles to remove stale assets without adding unrelated UI capabilities.

## Review of the moved tests

`crates/ink-compiler/src/native-lists.test.ts` was removed. All original behavioural cases remain covered:

- The seven original ineligible cases now assert exact source preservation rather than only whether the source changed.
- Both shadowing cases already asserted exact preservation and are retained.
- The alias case now executes the generated projector and checks the resulting template host type.
- The eligible Text case is consolidated into the stronger nested binding/projector case, which asserts an explicit text result and several independent property bindings.
- The eligible href Row case now checks projected title data, its template event binding, and deferred access to real navigation.

Expectations come from explicit sample data and public behaviour; no compiler or Ink helper is mocked. The execution harness resolves genuine workspace modules, emits temporary JavaScript using TypeScript, and removes it after loading. Filesystem fixtures are temporary and cleaned up.
