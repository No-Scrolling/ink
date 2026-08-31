# Perry pipeline lessons for Ink

Research date: 31 August 2026. Sources are limited to Perry's repository at commit [`9ee99197`](https://github.com/PerryTS/perry/tree/9ee99197cf6696caa3705679c7105d8ab1bb5697).

## Recommendation

Keep Ink's present pipeline. Perry is an ahead-of-time implementation of broad JavaScript and TypeScript semantics: SWC AST to HIR, transformation passes, LLVM object code and a linked runtime. Ink is a deliberately smaller language: Oxc TSX to typed UI IR, a versioned `app.ink`, a reusable Rust core and a retained wgpu renderer. Perry therefore supports the value of a strong intermediate representation, but its LLVM, garbage collector and native-widget architecture would make Ink larger and less predictable without improving Ink's LP3-specific goal.

There are five useful ideas to bring across, in this order:

1. Add compiler-emitted state-to-node dependency metadata so the retained engine can invalidate the smallest affected subtree.
2. Make one first-party module schema authoritative for compiler acceptance, TypeScript declarations and native capability selection.
3. Add an explainable build report for virtualisation decisions, capabilities and binary cost.
4. Key development caches from canonical lowered output plus every input that changes the result.
5. Borrow Perry's versioned native-binding contract only if Ink later permits third-party native extensions.

The first three are now implemented. `app.ink` version 2 carries layout and structural state-to-node bindings; substantial trees rematerialise affected retained branches while preserving conservative fallbacks. A single compiler schema now owns first-party package identity, API versions, runtime exports and base capabilities, with installed declarations validated against it. `ink info` exposes these bindings, list fast paths, capability causes, native costs and application-data size.

## How the pipelines differ

Perry's documented pipeline is `TypeScript -> SWC AST -> HIR -> transforms -> LLVM -> object file -> system linker`; its runtime implements dynamic JavaScript values, garbage collection and standard-library behaviour ([architecture](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/docs/src/contributing/architecture.md#L6-L40)). Its UI layer represents widgets as integer handles that map to platform-native widgets and dispatches operations through FFI ([architecture](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/docs/src/contributing/architecture.md#L42-L65)). On Android, for example, `Text` constructs and updates an Android `TextView` through JNI ([Android text backend](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry-ui-android/src/widgets/text.rs#L5-L53)).

Ink instead compiles a restricted declarative program into data. The application-specific tree, values, actions and assets live in `app.ink`; `ink-core` interprets them into a retained display list; and wgpu draws a consistent LP3 interface. This separation lets application changes avoid regenerating or recompiling Rust and lets one small native runtime serve every app.

That makes Perry's general AOT route an architectural mismatch for Ink:

- LLVM would compile application behaviour into each APK again, undoing the useful `app.ink`/runtime split.
- Supporting arbitrary TypeScript would require dynamic values, a garbage collector, closures, exceptions, async semantics and package compatibility. Perry's own architecture shows the runtime machinery this entails ([value and GC model](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/docs/src/contributing/architecture.md#L42-L61)).
- Android Views would surrender Ink's exact typography, spacing, input and LP3 rendering behaviour to platform widgets. Perry's handle registry also keeps a global reference for every widget ([Android widget registry](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry-ui-android/src/widgets/mod.rs#L41-L119)), whereas Ink can retain compact logical nodes and GPU buffers.

## Transferable ideas

### 1. Compile dependency metadata into `app.ink`

Perry's Android state runtime indexes concrete bindings by state handle. A state update visits only its bound text, slider, toggle, visibility and input widgets rather than searching the whole UI tree ([binding indices](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry-ui-android/src/state.rs#L12-L69), [targeted updates](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry-ui-android/src/state.rs#L138-L204)). Its compiler also recognises state reads in reactive text and lowers them to explicit subscriptions that update the existing widget ([reactive text lowering](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry-hir/src/lower/lower_expr/reactive_text.rs#L8-L89)).

Ink can apply the principle more cleanly because its language is restricted and state/value identities already exist. The compiler should optionally encode a compact dependency table such as:

```text
StateId -> dependent ValueId / NodeIdentity -> invalidation class
```

The invalidation class should distinguish:

- paint-only changes, such as text or colour;
- geometry changes that require measuring and laying out one ancestor branch;
- collection changes that invalidate one virtual list's row cache;
- navigation or capability state that changes the active screen.

The engine can retain its current scene-wide rebuild as a correctness fallback. A later renderer step can patch display-list ranges or GPU geometry only when the dependency table proves the rest of the scene unchanged. This is the most meaningful Perry-derived improvement for Ink's runtime pipeline.

Do not copy Perry's current Android list implementation. Its `LazyVStack` explicitly uses a render-all approach, and a count update clears the container and creates every row again ([Android `LazyVStack`](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry-ui-android/src/widgets/lazyvstack.rs#L1-L81)). Ink's fixed-geometry `ForEach` virtualisation is already the better design for the LP3.

### 2. Make first-party modules a single source of truth

Perry keeps an API manifest that its HIR lowerer consults to reject unsupported calls, its code generator checks against native dispatch, and its tooling uses to emit TypeScript declarations and API documentation ([API manifest](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry-api-manifest/src/lib.rs#L1-L17)).

Ink currently expresses one first-party operation across a declaration package, lowerer rules, capability detection and Android build selection. A small framework-owned module schema should describe the public function, request/result shape, protocol kind and capability dependencies once. The compiler can consume it directly and generate or validate the `.d.ts` and native build metadata. Keep exceptional lowering in Rust; the schema should remove repeated facts, not become a general code-generation language.

This is useful before third-party extensions because it makes adding or changing an Ink module safer while preserving the current author interface and zero runtime cost.

### 3. Make compiler decisions observable

Perry can emit a structured lowering report with selections, fallbacks, rejection reasons and the facts consumed by each decision ([lowering report model](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry/src/commands/compile/lowering_report.rs#L94-L206)). It can also attribute the final binary's code and data to crates and symbols, detect duplicate bodies or crate instances, and emit actionable suggestions ([size report](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry/src/commands/compile/size_report.rs#L1-L27)).

Ink's existing `ink info` is the right command surface; deepen it rather than add several new commands. It can explain:

- which `ForEach` instances use fixed-row virtualisation and why another fell back;
- which state changes are paint-only or geometry-affecting;
- why each capability was selected, including the source import or component;
- which Kotlin sources, Cargo features, permissions and dependencies that capability adds;
- the byte contribution of `app.ink`, native libraries, DEX, resources and assets.

This improves DX without adding authoring concepts. It also makes performance part of the framework contract: a developer can see when a harmless-looking component shape leaves the fast path.

### 4. Cache semantic output, not just source files

Perry keeps a bounded in-process parse cache for development rebuilds, keyed by path and exact source bytes ([parse cache](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry/src/commands/compile/parse_cache.rs#L1-L24)). More importantly, its object cache fingerprints post-transform HIR rather than raw source, so comments or formatting changes that lower identically can reuse codegen output. The key also includes a hash of the compiler executable, target and codegen-affecting options to prevent stale artefacts ([object-cache rationale](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry/src/commands/compile/object_cache.rs#L1-L24), [key inputs](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry/src/commands/compile/object_cache.rs#L263-L328)).

Ink already writes `app.ink` only when its bytes change and lets Gradle/Cargo reuse native outputs. Preserve that simple path. If compiler or watch time becomes material, use separate fingerprints for:

- canonical `app.ink` bytes, which only require repackaging the application asset;
- the canonical capability manifest, which selects the native build graph;
- framework build identity, Rust/Android toolchains, target and profile, which select the reusable native artefact.

An in-memory per-module Oxc cache is a later optimisation. Oxc arena lifetimes and Ink's currently small screen graphs make it unnecessary until measurements show parsing or relinking dominates the development loop.

### 5. Keep native extensions behind a versioned seam

Perry separates binding packages from runtime internals with `perry-ffi`; packages declare a native-library manifest, ABI range, functions and per-target artefacts. The compiler checks the ABI range and maps TypeScript calls to declared `extern "C"` symbols ([binding architecture](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/docs/src/native-libraries/overview.md#L17-L84), [ABI and symbol mapping](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/docs/src/native-libraries/overview.md#L111-L154)). It also rebuilds its runtime and standard library with the smallest Cargo feature set matching imports, caching each feature-set build separately ([selective native build](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/crates/perry/src/commands/compile/optimized_libs.rs#L1-L12)).

Ink's capability manifest already captures the useful selective-linking half. Keep first-party modules on the existing typed resource/action/controller seam. If Ink later opens native extensions to third parties, introduce a distinct versioned extension contract containing:

- supported Ink ABI range;
- resources, actions and controllers with their typed request/result schemas;
- required capabilities, permissions and Android components;
- target artefact and checksum;
- cancellation, timeout and thread-affinity declarations.

Validate the manifest against the native artefact before packaging. Do not expose `ink-core` structs directly, and do not confuse this native ABI version with the `app.ink` wire-format version. Until there is a genuine external native extension, the manifest and ABI machinery would be speculative weight.

## Performance evidence worth borrowing

Perry's current benchmark artefact retains raw samples, correctness status, runtime metadata and a fixed expected benchmark set; its documented refresh also checks source/harness freshness and refuses incomplete evidence ([benchmark artefact and refresh contract](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/benchmarks/README.md#L122-L145), [artefact validation](https://github.com/PerryTS/perry/blob/9ee99197cf6696caa3705679c7105d8ab1bb5697/benchmarks/benchmark_gate.py#L107-L215)). Ink already has the more relevant device-specific controls: physical LP3 runs, thermal gating and Android frame data. The remaining useful addition is provenance in every result: Ink commit, benchmark-harness hash, `app.ink` hash, capability-manifest hash, APK hash and resolved toolchain versions. That makes a regression traceable to the exact compiler, application data and native runtime that produced it.

## Suggested priority

| Priority | Change | Why |
| --- | --- | --- |
| High | Encode state/value/node dependencies and invalidation classes in `app.ink` | Extends Ink's retained renderer from scene-level reuse to safe subtree-level work avoidance. |
| High | Define one first-party module schema | Prevents declarations, lowering and Android capability selection from drifting. |
| High | Deepen `ink info` with fast-path and capability explanations | Makes the performance model understandable without expanding the authoring API. |
| Medium | Add APK size attribution and benchmark provenance | Turns size/performance budgets into evidence that identifies a cause. |
| Medium-low | Add semantic and per-module development caches | Valuable only if compiler/watch measurements, rather than Gradle packaging, justify the complexity. |
| Later | Define a stable external native-extension ABI | Necessary before third-party native code, unnecessary for today's first-party modules. |
| Avoid | LLVM codegen, general TypeScript execution, a JavaScript runtime or Android Views as Ink's renderer | These solve Perry's much broader problem and would undermine Ink's small, deterministic LP3 runtime. |

The main lesson is to adopt Perry's compiler discipline, not its execution model. Ink's restricted TSX and versioned application data are advantages: they allow stronger dependency analysis, better explanations and more aggressive reuse with substantially less machinery.
