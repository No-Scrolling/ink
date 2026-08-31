# Oxc tooling for Ink's TSX compiler

Research date: 30 August 2026. Sources are limited to Oxc's official documentation and repository.

## Recommendation

Keep the present compiler architecture. Ink already uses the most valuable part of Oxc: its allocator, TypeScript/TSX parser, AST and compiler-configured semantic pass. TSX should continue to lower directly into Ink's small intermediate representation, which is encoded as the versioned `app.ink` consumed by the native runtime.

The next useful Oxc adoption is its diagnostic model and renderer. After that, use semantic symbol/reference identities when Ink permits aliases, more local bindings, nested callbacks or shared modules. Add `oxc_resolver` only when imports extend beyond Ink's deliberately narrow relative screen modules into a package extension model. Do not put the transformer, minifier or JavaScript code generator in the compilation path: all three solve JavaScript-to-JavaScript problems, whereas Ink consumes the original JSX and TypeScript structure and emits Rust.

This keeps Oxc as a deep implementation detail of `ink-compiler`. Compiler-side additions affect Ink CLI dependency/build weight, not an app's APK, because no Oxc code is linked into the Android runtime.

## How Ink compiles TSX today

Ink is not translating arbitrary TypeScript into equivalent Rust statements. `App.tsx` and its local screen modules form a typed declarative language with TypeScript syntax:

1. Ink follows extensionless relative imports to local `.tsx` screen modules. For each file, `SourceType::from_path` identifies TSX, one Oxc `Allocator` owns the compilation unit, then `Parser` creates an Oxc `Program` AST and parser diagnostics ([Ink source compiler](../../crates/ink-compiler/src/source.rs), [Oxc parser](https://oxc.rs/docs/guide/usage/parser.html), [Oxc architecture](https://github.com/oxc-project/oxc/blob/main/ARCHITECTURE.md#foundation-layer)). Oxc's parser supports JS/JSX and TS/TSX; its arena architecture keeps a compilation unit's AST in one allocation arena.
2. Ink rejects parser errors, then runs `SemanticBuilder::new_compiler()`. Oxc defines this preset as semantic construction with AST-node storage disabled and syntax-error checking enabled ([Oxc source](https://github.com/oxc-project/oxc/blob/main/crates/oxc_semantic/src/builder.rs#L431-L434)). This matters because Oxc documents that parsing can recover to an AST while some expensive syntax checks live in semantic analysis ([parser return contract](https://github.com/oxc-project/oxc/blob/main/crates/oxc_parser/src/lib.rs#L195-L221)). Ink currently keeps only its diagnostics and discards the resulting semantic model.
3. Ink's handwritten lowerer validates a deliberately small grammar: named imports from `"ink"`, default imports of local screens, one default function, state declarations, supported JSX primitives, supported expressions and supported actions. It pattern-matches Oxc AST nodes and produces Ink's own typed `App`/`Node`/`Action` IR ([lowerer](../../crates/ink-compiler/src/lower.rs), [IR](../../crates/ink-compiler/src/ir.rs)). Local screens are expanded and their state identifiers are rebased into the linked app. This is the semantic centre of the framework.
4. Ink converts that IR into an owned, versioned wire model and encodes it as `app.ink`. Oxc's JavaScript code generator is not involved.
5. Cargo/Gradle compile the shared `ink-core`, renderer and Android adapter. Android loads `app.ink` as an asset and core validates and decodes it before creating the engine. There is no JavaScript engine, React runtime or JS/native bridge in the APK. The `packages/ink` package contains declarations for TypeScript/editor checking, not production JavaScript ([architecture](../architecture.md)).

The resulting shape is:

```text
App.tsx + local screens
  -> per-file Oxc parse + syntax/semantic diagnostics
  -> Ink validation, screen expansion and typed IR lowering
  -> compact app.ink encoding
  -> native Android build
```

## Tool-by-tool fit

| Oxc component | Fit for Ink | Recommendation | Migration cost |
| --- | --- | --- | --- |
| Allocator, parser and AST | Exact fit; already used | Keep. They preserve the TSX structure that the Ink lowerer needs. | None |
| Semantic builder, symbols and references | Already used for additional syntax checks, but its output is discarded. Oxc semantic data contains scopes, symbols and resolved references ([official source](https://github.com/oxc-project/oxc/blob/main/crates/oxc_semantic/src/lib.rs#L34-L61)). | Keep `new_compiler()` now. Later pass semantic identities into lowering to replace name-based state/item/import lookups once the grammar allows shadowing, aliases, helpers or nested callbacks. It is not a TypeScript type checker; Oxc's type-aware linting uses the TypeScript Go compiler separately ([official type-aware docs](https://oxc.rs/docs/guide/usage/linter/type-aware.html)). | Medium because the lowerer currently keys bindings by strings and semantic APIs need node/reference plumbing. Not justified by today's restricted grammar. |
| Diagnostics | Strong fit. Oxc diagnostics support severity, labels, multiple labels, help, notes, codes and source-aware reporters ([diagnostic type](https://github.com/oxc-project/oxc/blob/main/crates/oxc_diagnostics/src/lib.rs#L390-L517), [reporters](https://github.com/oxc-project/oxc/blob/main/crates/oxc_diagnostics/src/reporter.rs)). | Adopt next. Convert `CompileError` into `OxcDiagnostic` (or a thin Ink wrapper around it) and render parser, semantic and Ink errors through one path. This removes Ink's manual single-line renderer and enables precise multi-span errors such as mismatched object fields or invalid bindings. | Low to medium; the lowerer already attaches an Oxc `Span` and optional help to every Ink error. |
| AST visit / visit-mut | Oxc generates typed visitors for complete JS/TS ASTs ([visitor exports](https://github.com/oxc-project/oxc/blob/main/crates/oxc_ast_visit/src/lib.rs)). | Do not rewrite the lowerer around a generic visitor. Lowering is context-sensitive and naturally recursive: imports, state bindings, map-item bindings and expected node kinds flow down each call. Use a visitor only for a future independent whole-program scan that does not belong in lowering. | High for a lowerer rewrite with little benefit; low for an isolated scan. |
| Transformer | Poor fit. Oxc's fixed transform pipeline strips TypeScript, transforms JSX to JavaScript and lowers newer JavaScript syntax ([official transformer pipeline](https://oxc.rs/docs/guide/usage/transformer#features)). | Do not use before Ink lowering: it would erase the type annotations and JSX structure Ink consumes. Do not use after lowering: the output is Ink IR/Rust, not JavaScript. If Ink supports a new expression form, add it deliberately to the Ink grammar rather than normalising arbitrary JavaScript first. | High and architecturally counterproductive. |
| Resolver | Ink's local screen graph only needs deterministic relative `.tsx` resolution inside the app. Oxc Resolver implements the broader Node.js-style rules needed for packages ([official resolver docs](https://oxc.rs/docs/guide/usage/resolver.html)). | Add alongside the package extension model. Keep resolving, loading and graph validation inside the compiler's source-module implementation; a resolver finds files but is not a bundler or runtime. | Medium for resolution alone; high for the complete package model Ink would actually need. |
| Linter / Oxlint | Useful as general app-author tooling, not as the Ink compiler. Oxlint provides general JS/TS/TSX correctness rules and optional type-aware linting ([official linter docs](https://oxc.rs/docs/guide/usage/linter.html)). Ink's domain restrictions require compiler diagnostics, not lint rules. | Do not embed the Oxc linter or make it part of the minimal CLI yet. Keep `tsc --noEmit` for declaration/type checking and `ink check` for the Ink language. Consider optional Oxlint only if real apps accumulate ordinary TypeScript logic. | Low as an external dev command; high and unnecessary as embedded compiler policy. |
| Minifier | No fit. It optimises and shortens JavaScript, including dead-code elimination, mangling and whitespace removal ([official minifier docs](https://oxc.rs/docs/guide/usage/minifier)). | Do not add. Rust/Cargo release optimisation and Ink's conditional native features determine APK size. | None because it should be skipped. |
| JavaScript codegen | No fit in the production pipeline. Oxc codegen prints an Oxc AST back to JavaScript and can produce source maps ([official source](https://github.com/oxc-project/oxc/blob/main/crates/oxc_codegen/src/lib.rs#L131-L171)). Ink encodes its own IR rather than emitting JavaScript. | Oxc codegen would only be useful for a future source-to-source formatter/fixer, not the `app.ink` compiler. | High if misapplied; zero if skipped. |
| Formatter / Oxfmt | Potentially pleasant authoring DX, but orthogonal to compilation. Oxfmt supports native TSX formatting and a Prettier-compatible workflow ([official formatter docs](https://oxc.rs/docs/guide/usage/formatter.html)). | Leave formatting to the editor or an optional app dev dependency. Do not bundle it into `ink check` while Ink is optimising for a small command surface. | Low as optional tooling; ongoing distribution/configuration cost if bundled. |
| Language server | Not a near-term shortcut. Oxc's server manages workspaces, documents, configuration and LSP transport, while tools remain responsible for diagnostics and edits ([official language-server design](https://oxc.rs/docs/contribute/language_server)). | First make Ink diagnostics structured. If compiler feedback on each edit becomes important, expose the compiler through an editor task or a small Ink-specific LSP later. TypeScript already supplies completions from `packages/ink`; an Ink server would chiefly add dialect validation and custom actions. | High: document caching, incremental compilation, diagnostic publishing and editor packaging are a product of their own. |

## Suggested sequence

1. Preserve the current direct `Oxc AST -> Ink IR` compiler.
2. Unify diagnostics on `OxcDiagnostic` and its source-aware renderer.
3. Continue adding framework behaviour to the small, versioned Ink IR, not to transformed JavaScript.
4. Use semantic `SymbolId`/reference data when a concrete language feature makes string binding ambiguous.
5. Design the package extension interface first; only then introduce `oxc_resolver` beneath the existing source-module implementation.
6. Keep Oxlint and Oxfmt optional developer choices unless app code grows enough that a framework-wide default earns its weight.

The important architectural benefit is not using every Oxc crate. It is using Oxc as a reliable, fast TSX front end while Ink owns a much smaller language and native runtime. That restricted, serialised middle layer is what makes small APKs, reusable native builds and predictable behaviour possible.
