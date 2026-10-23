---
title: "Remaining performance research opportunities"
description: "Recent literature mapped to remaining Ink bottlenecks after the artwork cache and cooperative yielding changes."
---

The strongest architectural bet is an authoritative transactional data graph that carries changes through derived views into native row plans. Take the deferred collection source towards that design first, with an explicit gate on Android tail latency. IRIDIUM is the strongest new QuickJS-specific research lead; evaluate it on a bounded module before considering app-wide use. Reusable glyph slots offer a smaller renderer experiment. Paper results do not predict Ink app speed-ups; the small QuickJS experiment below measures only its stated data path.

This review inspects `dfef70ed9f2d5564914c868ff332eaa98c288b83` on 22 October 2026. It prioritises publications from 22 October 2024 to 22 October 2026. Three research subagents cover rendering, runtime/startup and incremental computation; a fourth synthesises bolder architectures. The parent checks mobile responsiveness, verifies findings and runs the small experiments below. Exa supplies discovery and primary-page retrieval, alphaXiv supplies discovery and PDF reading, and web search verifies proceedings, author manuscripts and artefacts. Production code is unchanged. No device run was performed.

## What has already changed

The [measured follow-up](recent-performance-results.md) selected adaptive artwork retention and `yieldToInput()` for production. Eligible custom native rows, release QuickJS bytecode, retained Vulkan resources and pipeline caching also exist. The collection source and route-preparation prototypes were deferred. Bounded retained layout, reconstruction-cost cache weighting and SDK-wide React Compiler coverage were removed after measurement. These are the starting conditions for this review.

## Production order

The order reflects current evidence and the cost of reaching a useful decision. It does not rank predicted percentage gains.

| Order | Opportunity | What is new in this review | First step | Effort |
| --- | --- | --- | --- | --- |
| 1 | Authoritative collection changes | Updated production assessment of a deferred prototype | Isolate Android tail latency and remaining native copies | Medium–large |
| 2 | Incremental derived data | Enzyme and Differential Execution extend the producer-side approach | One explicit filter/aggregate pipeline with full-refresh fallback | Medium for a narrow pipeline |
| 3 | Reusable bitmap glyph slots | A concrete allocator implementation lead; engineering article | Capture atlas churn, then compare at an equal byte budget | Medium |
| 4 | Better QuickJS bytecode through IRIDIUM | Direct OOPSLA 2026 QuickJS backend and public artefact | One real module after current Oxc/Bun passes; measure coverage first | Medium–large |
| 5 | A bounded asynchronous compute kernel | QJWasm's October 2026 runtime/boundary design | Compare chunked JS, a Rust worker and portable Wasm only if justified | Medium for one Rust kernel |
| 6 | Visibility-aware work attribution | FSE 2026 detection method | Correlate commits/layout with submitted output before changing behaviour | Small diagnostic step |
| 7 | Dependency-aware retained layout | Known PLDI direction remains unfinished | One container with retained scene and hit-region outputs | Large |
| 8 | Interpreter specialisation | Weval supplies a published mechanism and available source | Profile dispatch, then establish native QuickJS feasibility | Very large |

Collection changes and derived data share a foundation. Count their end-to-end benefit once. The glyph allocator is a practical companion, not a peer-reviewed result. No remaining recommendation has a verified physical Light Phone III speed-up.

## Small QuickJS experiment

A throwaway prototype checks the missing change boundary in the actual Ink runtime. The full path replaces one record in an immutable array, filters eligible records and recomputes 16 group counts/sums. The delta path replaces that record in a persistent 32-way tree and maintains the same aggregate, publishing new immutable result arrays. This tests an algorithm and ownership change in JavaScript; it does not depend on moving computation to Rust.

The release runtime was freshly built from the current checkout with its pinned QuickJS-ng and 64 MiB heap limit. The host is an Apple M4 Pro, arm64, Darwin 27.2.0. Four fresh processes alternate which path runs first. Each size/path has 16 discarded warm-ups and 128 measured sparse updates per process. Expected outputs use a separate numeric array and direct full-loop oracle outside timing. Every output is checked, including filter crossings; retained earlier aggregate and record snapshots must remain unchanged. Dense replacement uses full reconstruction in both paths, including building the persistent index where required.

| Records | Full sparse edit, ms | Delta sparse edit, ms | Full replacement, ms | Delta-path full replacement, ms |
| --- | ---: | ---: | ---: | ---: |
| 1,000 | 0.0603 | 0.00221 | 0.153 | 0.170 |
| 10,000 | 0.556 | 0.00258 | 1.519 | 1.679 |
| 100,000 | 5.611 | 0.00317 | 15.287 | 17.174 |

Values are nearest-rank medians of the four process medians. Full-replacement values use eight measurements after two warm-ups per process. Initial construction is timed separately and retained in the raw results. At 100,000 records, the four sparse full-path medians range from 5.585 to 5.766 ms; delta medians range from 0.00296 to 0.00329 ms. The small absolute delta timing includes timer-call overhead.

**Decision.** Authoritative changes can remove material producer work in a controlled large-data workload while preserving old snapshots. The persistent representation adds construction/dense-replacement cost: this probe's 100,000-record replacement is about 12% slower. That supports a bounded source/view experiment with an explicit dense-update decision. It does not establish a general SDK win or validate native graph integration. React, bridge transfer, row projection, layout, GPU work, Android scheduling, memory use and input tails are outside this probe. No gains from those stages are implied.

Source, runnable scripts, the saved runtime executable, SHA-256 manifest and raw samples remain under `.test-output/research/2026-10-22/` as `derived-probe.js`, `run-probes.ts`, `derived-manifest.json`, `derived-raw.json` and `derived-summary.json`. No production source or project test suite was changed for this experiment.

### Production-path scaling check

The saved selected release build was also replayed through the existing complete QuickJS/React/commit/native-scene harness. Its executable hash matches the selection manifest, and changed production/workload source hashes match the current checkout. Three processes each run 40 measured samples after eight warm-ups. All 30 existing workload contracts and idle observations pass in each process.

| Rows in the ordinary array list | Sparse edit to ready scene, ms | Native apply, ms |
| --- | ---: | ---: |
| 100 | 0.164 | 0.0133 |
| 1,000 | 0.205 | 0.0144 |
| 10,000 | 0.616 | 0.0162 |

These are medians of three process medians. Every size projects one changed row, with one operation, no create/remove and a 205-byte diagnostic serialisation. The growing total with nearly constant native application supports investigating work before native application. This observation does not isolate the item scan from producer copying, allocation, queueing or other JS work. It is a scaling check of the current path, not a before/after comparison with the data-only prototype above. It excludes Android, GPU submission and display presentation.

`run-host.ts`, `host-manifest.json`, `host-summary.json` and the three `host-*.json` files preserve the replay. The saved fixture and binary locations are recorded in the manifest. Timings ran sequentially, after compilation and separately from the data-only probe.

## Architectural bets

These are proposed Ink designs, informed by the papers; none is an architecture supplied or validated by those papers.

### Own the data and its derived views once

Let Rust own canonical keyed records, persistent revision roots, ordered indexes and a bounded graph of pure derived operators. JavaScript owns commands, fetching policy, effects and arbitrary React state. React observes immutable revision handles through `useSyncExternalStore`; it materialises only explicitly requested records. Native lists bind a view handle directly to an existing native row plan. This can remove the duplicated arrays, whole-value discovery and repeated projection currently between producer and scene.

The proposed boundaries are a typed source, a transaction carrying `baseRevision` and keyed operations, a pure operator plan with revisioned parameters, and an explicit snapshot read. A native transaction publishes source records, derived membership/order and bound rows atomically. That atomicity does not extend to unrelated React commits. New actions carry stable keys and a mount/view generation; current-record reads are explicit. Keep existing array lists and item callbacks on their current compatibility path until migrated.

Start with stable-key replacement and one filter/count view for a real data-heavy screen. Preserve old snapshots, cap retained history and recover from revision gaps. Do not publish failed persistence attempts or work from aborted React renders. Define ordering, comparison and nullable-value semantics explicitly; arbitrary JavaScript callbacks do not become native operators automatically. Add insertion, moves and more operators after the first milestone earns its cost. The QuickJS probe above demonstrates the producer mechanism only; it does not demonstrate that Rust ownership is necessary or profitable.

**Falsifier:** compare complete producer-to-submission tails, memory and construction against current arrays. Reject or narrow the design if index retention, snapshot access or cross-language synchronisation erases the saving. This includes the collection and derived-data candidates rather than adding another independent gain.

### Make layout outputs belong to retained fragments

Retain measurement, position, clip, drawing and hit-region outputs by stable identity. An ordered dirty-field scheduler updates dependencies; parent transforms move retained child fragments. Start with one vertical Stack island and variable-height text, keeping the existing whole-root fallback and full GPU redraw. A local change can then avoid allocating all unchanged scene/hit records. Merely reusing measurements leaves much of that work intact.

The renderer must eventually consume retained fragments without flattening every fragment into a fresh full scene. Width changes, font/image completion, portals, focus, scroll anchors and painter order remain explicit dependencies. Reject this bet if bookkeeping, unavoidable sibling movement or final flattening dominates. This is a larger refactor than the removed layout cache and should be evaluated as such.

### Introduce a selective compiler tier

Keep values, GC, callbacks and jobs in QuickJS. Let the build pipeline select eligible modules for an IRIDIUM-compatible front end with a pinned-engine fingerprint and whole-module fallback. Only after better bytecode pays off should a selective native tier be considered. Weval does not supply the required AArch64 backend. This route attacks remaining execution cost while leaving application representations and layout ownership intact; it is a competing investment when profiling identifies interpreter work as dominant.

## Carry authoritative collection changes through to the scene

**Publication.** *Stateful Differential Operators for Incremental Computing*, POPL 2026, [DOI 10.1145/3776728](https://doi.org/10.1145/3776728). *DeCo: A Core Calculus for Incremental Functional Programming with Generic Data Types*, OOPSLA1 2026, [DOI 10.1145/3798264](https://doi.org/10.1145/3798264). These are known leads from the earlier review.

The useful mechanism is an explicit input change with operator-maintained state, rather than discovering every change by scanning a new full value. The first paper formalises stateful differential operators; DeCo extends cached incrementalisation to generic data types. Neither supplies an ordered React collection API. DeCo's interpreter evaluation is a proof of principle.

**Current seam.** [`replaceInPlace()`](../../packages/ink/src/native-list.ts) scans all item identities and copies the projected array after a change. Rust's [`NativeList`](../../crates/ink-core/src/react/native_list.rs) can copy shared per-item versions through `Arc::make_mut`. A sparse bridge message already exists; changing transport alone does not remove these costs.

**Production proposal.** Recover the deferred immutable collection prototype and initially support replacement at a stable key. Carry base revision, next revision and changed keys from producer through projection, native storage and row invalidation. Preserve shared snapshots without converting them back to full arrays. Keep the array API and full-snapshot recovery. Extend to insertion, removal and moves only when their ordered semantics and storage costs are explicit.

**Existing Ink evidence.** The [prototype](recent-performance-results.md#collection-updates) reduced a one-item edit among 10,000 rows from 0.608 to 0.176 ms at the host median. It did not make the entire pipeline logarithmic. In the earlier combined Android candidate, a 5,000-row edit improved at the median but worsened at p95, from 16.088 to 19.308 ms. That combined result does not isolate the collection change.

**Release gate.** Compare the collection change alone with current `dfef70e`, including producer updates, allocations, native bookkeeping and scene submission. Use sparse and dense edits, stale histories, changing captured state, ordinary arrays and small collections. Require stable old snapshots, current-item callbacks, bounded history and Android p95/p99 results that justify the trade-off. Effort: medium to large. Confidence: high in the remaining O(n) work; conditional in the production latency benefit.

## Preserve changes through filters, aggregates and projections

**Publications.** Ritwik Yadav and colleagues, *Enzyme: Incremental View Maintenance for Data Engineering*, SIGMOD Companion 2026 industry paper, published 30 May. [Proceedings](https://doi.org/10.1145/3788853.3803098), [author manuscript](https://arxiv.org/abs/2603.27775). Prashant Kumar, André Pacak and Sebastian Erdweg, *Incremental Computing by Differential Execution*, ECOOP 2025, published 25 June. [Proceedings](https://doi.org/10.4230/LIPIcs.ECOOP.2025.20), [evaluated MIT-licensed artefact](https://doi.org/10.4230/DARTS.11.2.13). Both are new leads relative to the previous Ink shortlist.

Enzyme maintains derived data across pipeline stages, consolidates cancelling changes and chooses incremental or full refresh using workload costs. Its TPC-DI evaluation favoured incremental refresh on six of eight datasets; a dataset rewriting over 95% of rows favoured full refresh. These are distributed data-engineering workloads, not mobile measurements. No reusable Enzyme source licence was verified.

Differential Execution propagates changes through statements, branches and loops while caching unaffected computation. Its Bellman–Ford evaluation is narrow: reported update families did not change the shortest-path outputs, and the table model assumes fixed indices. Its verified language and Scala interpreter are not JavaScript. These limits make a constrained operator API more credible than automatic incrementalisation of arbitrary app code.

**Current seam.** [`createStore()`](../../packages/store/src/index.ts) decodes and publishes whole values; [`resource()`](../../packages/ink/src/resource.ts) publishes whole loaded results. Native lists cannot recover producer changes after app code has already copied, filtered or sorted the collection. The earlier collection prototype did not maintain these upstream derivations.

**Production proposal.** Start with explicit pure keyed `filter`, computed-field and group-count operators over the collection source. Consolidate repeated writes within one logical transaction. Publish one immutable revision after all dependent stages finish. Keep full recomputation for large change sets, global predicate changes and unsupported operations. Measure the crossover before introducing a cost model. Ordered views need a stable index and deterministic ties; database multiset semantics alone do not provide UI ordering.

**Release gate.** Use a real expensive app transformation, with sparse edits that change output, no-op edits, filter-boundary crossings, sort moves and dense replacement. Include first construction, retained indexes, total producer-to-scene latency and input tails. Preserve store retry semantics: failed writes and aborted renders cannot mutate committed incremental state. Effort: medium for one explicit pipeline, large for a general SDK. Confidence: medium until a representative producer cost is established. This extends the collection work and must not be counted as an additive independent speed-up.

## Optimise JavaScript before emitting QuickJS bytecode

**Publication.** Meetesh Kalpesh Mehta, Anirudh Garg, Aneeket Yadav and Manas Thakur, *IRIDIUM: A Framework for Statically Optimizing JavaScript Programs*, OOPSLA1 2026, article 165, April. [Proceedings](https://doi.org/10.1145/3798273), [author manuscript](https://www.cse.iitb.ac.in/~manas/docs/preprints/oopsla26.pdf), [source/build artefact](https://zenodo.org/records/18942828). The artefact advertises CC BY 4.0; its dependencies retain their own licences. It was inspected through metadata, not downloaded or built.

IRIDIUM makes bindings, environments and effects explicit, removes redundant lexical checks/coercions, then selects existing QuickJS instructions. The interpreter and collector stay unchanged. Across 16 selected JetStream 2 programs, it reports 1.18× versus ordinary QuickJS compilation; its larger 2.5× figure compares with its own transpiled baseline. Some workloads regress. IRI passes about 90% of the relevant Test262 comparison against QuickJS, so an app-wide replacement is premature.

**Current seam.** [`compile()`](../../crates/ink-runtime/src/bytecode.rs) currently uses QuickJS's compile-only source path. Oxc React compilation and Bun bundling/minification already precede it in [`bundle-javascript.js`](../../crates/ink-compiler/src/bundle-javascript.js). IRIDIUM would be an alternative build-time front end, rather than another request to ship bytecode or a drop-in minifier.

**Production proposal.** Reproduce one relevant pure, closure-heavy or numeric app module with the released loader. Retarget it to Ink's exact pinned QuickJS-ng build and compare the same post-Oxc/Bun input. Establish eligible syntax and semantic coverage before timing. Keep the existing compiler for entire unsupported modules; do not mix incompatible closure representations or remove checks based only on observed data.

**Release gate.** Preserve lexical errors, property/coercion side effects, evaluation order, closures, exceptions and async behaviour. Measure total module execution, startup evaluation, input tails, bytecode/APK size, memory and build time. Stop if gains disappear after today's compiler passes or only reverse overhead introduced by IRIDIUM's own lowering. Effort: medium to large. Confidence: high in the direct runtime match, medium in published evidence, low in immediate deployability. This is the strongest new engine-specific experiment, ahead of a complete AOT backend.

## Move a measured computational kernel off the JavaScript thread

**Publication.** Yunfei Hu, Fei Gu, Puyu Han, Zhihao Wei and Zhijun Li, *QJWasm: A lightweight runtime system for efficient WebAssembly execution in resource-constrained environments*, Journal of Systems Architecture 179, October 2026. [Publisher](https://doi.org/10.1016/j.sysarc.2026.103926), [QJWasm source](https://github.com/LazyBoy-KK/QJWasm), [WA2X fork](https://github.com/LazyBoy-KK/wa2x_for_qjwasm). Full-paper tables were inaccessible; publisher abstract/section snippets and source were checked. Treat its performance claims as less thoroughly verified than IRIDIUM or Weval.

QJWasm coordinates requests, callbacks and Promise results between QuickJS and a Wasm worker, with lifetime integration. Its compute gains include moving code to Wasm; they do not mean unchanged JavaScript becomes faster. The public implementation uses Rust/rquickjs, but custom forks, a local runtime dependency and unresolved build instructions remain. QJWasm's manifest declares MIT; no root licence file was verified. The WA2X fork is Apache-2.0. No successful Android build was established.

**Current seam.** [`yieldToInput()`](../../packages/ink/src/scheduling.ts) already improves responsiveness for chunked work while leaving the calculation on the JS thread. Ink's [`binary.rs`](../../crates/ink-runtime/src/binary.rs) and native reply queue provide ownership conventions, but a computational worker needs its own bounded request path. Store, crypto and platform I/O already have native implementations.

**Production proposal.** Choose one costly real parser/import/data kernel. Compare chunked JavaScript with a Rust worker implementing the same algorithm. Add packaged AOT Wasm only if reusing portable third-party kernels justifies its runtime and integration cost. Borrow the boundary design rather than adopting the whole QJWasm fork. This is an engineering adaptation, not a reproduced paper result.

**Release gate.** Include argument/result copies, batching, cancellation, throughput, input p95/p99, retained bytes, APK size and memory. Keep QuickJS access and Promise settlement on the owning thread. Bound outstanding jobs and ignore cancelled generations safely. Audit the artefact's unsafe cross-thread assumptions before reuse. Effort: medium for a narrow Rust kernel, large for a public Wasm facility. Confidence: medium in the mechanism, low in turnkey artefact readiness. It complements cooperative yielding only for workloads worth moving.

## Retain layout dependencies and outputs

**Publication.** Marisa Kirisame, Tiezhi Wang and Pavel Panchekha, *Spineless Traversal for Layout Invalidation*, PLDI 2025, [DOI 10.1145/3729322](https://doi.org/10.1145/3729322). This is a carry-forward architectural lead.

The algorithm queues dirty layout fields in dependency order and avoids walking clean auxiliary nodes. Its 1.80× mean result compares generated incremental engines over 2,216 browser-layout benchmarks. It does not compare full Ink frames.

**Current seam.** [`relayout_scene_inner()`](../../crates/ink-core/src/lib.rs) clears scene, text, hit-test and related collections, then lays out the root. [`fixed_text_patches()`](../../crates/ink-core/src/react.rs) already bypasses this for eligible fixed-width text. A new design must target local size-changing updates that miss that fast path.

**Production proposal.** Start with one bounded container. Track measurement, position, paint and hit-region dependencies separately; retain unaffected scene fragments. Propagate changed sizes until dependent results stabilise. Order structural invalidation explicitly. A measurement cache by itself leaves scene reconstruction and traversal costs intact.

**Release gate.** Compare total scene readiness for local geometry changes and dense resize. Record visited fields, allocations, retained memory and hit-region construction. Preserve clipping, scroll anchors, input geometry and font-scale changes. Compare saved output pixels before making any visual-equivalence claim. The [removed bounded-cache trial](recent-performance-results.md#removed-trials) improved a local edit slightly but slowed full resize; it did not implement Spineless traversal or retained dependent scene fragments. Effort: large. Confidence: high in the code-path match, medium in net benefit.

## Attribute visual work that never contributes to a frame

**Publication.** Zhihao Lin, Mingyi Zhou, Bo Sun, Han Hu, Gang Fan and Li Li, *Phantom Rendering Detection: Identifying and Analyzing Unnecessary UI Computations*, FSE 2026. [Proceedings](https://doi.org/10.1145/3797101), [author manuscript](https://mathieu0905.github.io/papers/hapray_fse.pdf), [Apache-2.0 artefact](https://github.com/SMAT-Lab/PhantomRendering). Published 30 June 2026, with a July issue date. This is a new lead for Ink's research notes.

The paper correlates UI computation with rendering outcomes and attributes retired CPU instructions to functions. It found problematic cases in eight of 22 OpenHarmony apps, with up to about 40% instruction waste in an operation. The tool detects and localises suspected waste; it is not a general runtime optimisation. The implementation depends on OpenHarmony, and the paper acknowledges that necessary state work may have no immediate visual output.

**Current seam.** Ink already uses [`Activity`](../../packages/ink/src/file-router.tsx) for covered routes, excludes hidden nodes from native rendering, checks visible animations in [`has_scene_animations()`](../../crates/ink-core/src/lib.rs), and schedules frames on demand in [`MainActivity.kt`](../../platform/android/app/src/main/kotlin/com/vandam/ink/MainActivity.kt). The [host idle contract](../../test/performance/main.rs) already requires no settled-scene commits or measurements. No current phantom-rendering defect has been demonstrated.

**Production proposal.** Correlate JS commits and native apply/layout work with frame requests and submissions during active, covered and revealed routes. Inspect shared-store updates, native subscriptions and bursty input. If evidence identifies redundant pure visual derivations, defer or combine them before projection/layout. Preserve state transitions, ordered events, editor acknowledgements, background services and fresh state on reveal. This adaptation is an engineering inference; the paper does not implement it for Ink.

**Release gate.** Measure CPU work, allocations, input p95/p99, wasted derivations and reveal latency. Keep useful data work distinct from visual preparation. Identical screenshots or absent submissions alone cannot establish semantic redundancy. Effort: small for diagnosis, medium for a confirmed fix. Confidence: high in the diagnostic value, low in residual benefit before profiling.

## Reclaim bitmap glyph slots before changing rasterisers

**Source and status.** Rubén Osorio López, [*Rendering Crispy Text On The GPU*](https://osor.io/text), 11 June 2025. This is a first-party engineering article, not a peer-reviewed paper. It describes reusable atlas allocation using Morton-order bitsets and power-of-two cells. It supplies implementation detail but no comparative latency study; a reusable implementation licence was not verified. Treat the allocator mechanism as an engineering companion to the academic shortlist.

**Current seam.** Ink's grayscale glyphs and masks share a monotonically allocated shelf in [`compact.rs`](../../crates/ink-renderer-vulkan/src/compact.rs). Exhaustion clears cached entries and rebuilds prepared geometry before growing the texture. The separate colour/system atlas already reclaims whole unused pages; it does not reuse dead slots within a surviving page. Artwork admission changes do not address either allocator.

**Production proposal.** Keep the existing rasteriser and coverage pixels. Experiment with individually reusable slots at the same texture budget. Pin every glyph and mask referenced by the complete prepared scene, free only after the frame fence, and invalidate prepared UVs when a slot is reused. Include character aliases, clear padding and preserve the existing live-set limit. Power-of-two cells may increase fragmentation, so occupancy matters as well as reset counts. The earlier review identified glyph eviction prerequisites; this article makes one allocator approach concrete rather than establishing a new Ink gain.

**Release gate.** Capture route/font-size/multilingual turnover before implementing a policy. Compare atlas resets, repeated rasterisation, uploads, preparation p95/p99, fragmentation and memory. Include a control where the entire live set nearly fills the atlas. Compare saved pixels with the unchanged rasteriser. Effort: medium for production integration. Confidence: high in the allocator limitation; conditional on real atlas churn. This is the smallest renderer experiment worth considering after that profile.

## Investigate interpreter specialisation as a longer-term option

**Publication.** Chris Fallin and Maxwell Bernstein, *Partial Evaluation, Whole-Program Compilation*, PLDI 2025, published 13 June. [Proceedings](https://doi.org/10.1145/3729259), [author manuscript](https://cfallin.org/pubs/pldi2025_weval.pdf), [Weval source](https://github.com/bytecodealliance/weval). The source uses Apache-2.0 with LLVM exceptions.

Weval specialises an interpreter against known bytecode, removing dispatch and some interpreter-state traffic. Its SpiderMonkey/Octane evaluation inside Wasmtime reported 2.17× relative to the portable interpreter with inline caches. The compiled Wasm module grew from 8 to 52 MiB, including the benchmark and inline-cache corpus. Those figures are neither a QuickJS comparison nor a native Android result. The available tool targets Wasm, which is a substantial mismatch for Ink.

**Current seam.** [`bytecode.rs`](../../crates/ink-runtime/src/bytecode.rs) already skips JavaScript parsing in release builds. `JS_EvalFunction` still executes the QuickJS interpreter. React Compiler and native-row compilation operate at different layers and do not eliminate all interpreter dispatch.

**Feasibility gate.** Profile representative released apps for interpreter dispatch and stack traffic first. If they dominate, investigate selective specialisation of a pure hot function while preserving interpreter fallback. Establish a native AArch64 build path before considering adoption; adding a Wasm runtime merely to run Weval could erase the benefit. Preserve GC roots, exceptions, cancellation, closures and job ordering. Measure whole-app startup, steady JS CPU, input tails, code size, memory and build time. Effort: very large. Confidence: high in the published result, low in near-term Ink deployment. This competes with the previously noted JWST AOT direction; the benefits are not additive.

## Audit optimisation of optional native dependencies

**Publication.** Han Hu and colleagues, *Source-Free Detection and Impact Analysis of Compiler Optimization Problems in Mobile Applications*, ASE 2026, 12–16 October. [Conference record](https://conf.researchr.org/details/ase-2026/ase-2026-research-track/10/Source-Free-Detection-and-Impact-Analysis-of-Compiler-Optimization-Problems-in-Mobile), [current author manuscript](https://arxiv.org/html/2606.23512v4). First preprint: 22 June; v4: 1 October. The manuscript identifies DOI `10.1145/3832783.3834340`.

OptDetect classifies native binary chunks to find low optimisation settings inside mixed libraries. It reports 81.9% real-world low/high classification accuracy. Its ecosystem prevalence and instruction-count improvements do not establish that Ink contains an affected library.

**Current seam and action.** [`Cargo.toml`](../../Cargo.toml) already requests speed optimisation and fat link-time optimisation; Android release builds use `--release`. Optional SDKs such as MapLibre arrive through separate dependencies in [`build.gradle.kts`](../../platform/android/app/build.gradle.kts). Inspect actual capability-specific APK libraries and provenance. Use owned build metadata first, then inspect unknown hot binaries if needed. Treat classifier results as leads, not proof of flags or latency impact. A confirmed dependency build problem would be a small, isolated fix; no such defect was found in this review. This is a build-quality check, not a ranked runtime optimisation.

## Watch list and rejected transfers

| Research | Publication status | Decision for Ink |
| --- | --- | --- |
| [Windfoil](https://arxiv.org/abs/2610.02468), Matt DesLauriers, 1 October 2026 | Recent preprint; peer-reviewed venue unverified. [Apache-2.0 display implementation](https://github.com/texel-org/windfoil-algorithm) | Closed-form analytic coverage is interesting for cold or large glyphs. The paper compares with Slug, not Ink's warm bitmap atlas, and its middle zoom range can be slower. Profile cold glyphs first; any new coverage requires a separate quality decision. |
| [The Scanline Sweeper](https://rookandpossum.com/posts/scanline-sweeper/), 27 June 2026 | Self-published paper, not traditionally peer reviewed; appendix code MPL-2.0 | Another analytic-glyph option, competing with Windfoil. Two-font desktop measurements do not establish a mobile win; the author expects bitmap sampling to be cheaper. |
| [Semantics for 2D Rasterization](https://doi.org/10.1145/3839471), Kulkarni, Whiting and Panchekha, October 2026 | Peer-reviewed OOPSLA2 paper; [current manuscript](https://arxiv.org/abs/2603.23696v2) and released artefact | The final paper reports 1.12× over Skia Graphite on 139 traces. Its layer/filter rewrites have few direct matches in Ink's flat renderer. Older abstracts report different figures. Use it as a correctness reference; do not claim its proofs certify Ink's blending or antialiasing. |
| [Tileink](https://github.com/Jianqoq/tileink), 2026 | MIT/Apache-2.0 engineering artefact; no peer-reviewed paper or Android validation verified | Stable node identities, output history and a bounded change journal provide a useful retained-damage contract. Ink clears full attachments, so this needs persistent output, conservative damage and a full-frame fallback. Swapchain images need individual histories; presentation-region hints alone do not preserve pixels. Large experiment. |
| [Exploiting Partial JPEG Decoding to Mitigate On-Device Image Processing](https://doi.org/10.1109/TMC.2026.3697605), Gong and colleagues, 2026 | IEEE TMC publication verified; full paper access blocked | The primary abstract targets DC features for collaborative inference, not exact UI display. Do not apply its reported gains to artwork. Independently, bundled JPEGs decode at source size while remote images support target sizes; display-resolution renditions are ordinary engineering to profile if oversized bundled assets exist. |
| [Global Pass Barriers Without Per-Resource RHI Tracking](https://arxiv.org/abs/2607.26506), July 2026 | Preprint, peer-reviewed venue unverified | Desktop independent-pass results do not justify removing Ink's required texture transitions. Ink already has a compact single-pass renderer; the study excludes Android tilers. |
| [LayoutOptimizer](https://doi.org/10.1016/j.scico.2025.103287), September 2025 | Peer-reviewed journal paper | Repairs Android View hierarchies. Ink uses Rust layout; removing an Ink wrapper needs a separate argument for constraints, identity, focus and hit behaviour. No direct transplant. |
| [DBSP extended paper](https://doi.org/10.1007/s00778-025-00922-y), May 2025 | Peer-reviewed extension of the 2023 work | Useful foundations for derived views. Current Ink database assets are read-only, so reactive SQL would require a new writable-data/changefeed contract. Do not add a server query engine to optimise today's ten-row query example. |
| [Integrating Static Optimization and Dynamic Nature in JavaScript](https://doi.org/10.1145/3742876.3742877), Saito and Iwasaki, GPCE 2025 | Peer-reviewed; reusable source/licence unverified | Preconstructs class shapes and specialises declared-field access in an older QuickJS. Its 13 prepared class benchmarks do not establish coverage of Ink's object-record renderer or emitted bundles. Count hot eligible accesses first; despecialisation overhead and engine-fork maintenance can erase the gain. |

Keep sparse mapped-buffer writes rejected unless a new workload changes the evidence: the earlier trial cut uploaded bytes but increased CPU time. Keep SDK-wide React compilation and the removed layout/cache trials removed. These [recorded results](https://github.com/No-Scrolling/ink/blob/dfef70ed9f2d5564914c868ff332eaa98c288b83/test/performance/README.md#costs-and-rejected-trials) are stronger evidence for current Ink than a related paper's headline result.

## Research coverage

Exa discovery requested **165 result slots across 21 searches and four workstreams**. These are requested slots, including duplicates, not 165 distinct papers or full-text reads. Three research subagents performed specialist searches; the parent added mobile responsiveness and coverage checks. A fourth subagent reviewed architectural options using the resulting primary research.

| Workstream | Exa searches | Requested result slots |
| --- | ---: | ---: |
| Incremental computation, compiler and data | 5 | 40 |
| Runtime, startup and compute boundaries | 5 | 40 |
| Rendering, glyphs and graphics | 5 | 40 |
| Mobile responsiveness and parent coverage checks | 6 | 45 |

Seven alphaXiv discovery calls covered the initial topics; primary full text and PDF questions then checked the shortlisted mechanisms. Web search and direct retrieval verified proceedings, author manuscripts, artefact licences and current platform APIs. Repeated papers were merged by mechanism. Engineering articles and preprints are labelled separately from peer-reviewed publications. Blocked full-text reads and unbuilt artefacts are stated beside their affected recommendations. This is a targeted sweep, not a claim of exhaustive coverage or a universal state-of-the-art ranking.

## Preserve measurement endpoints

Use the existing [performance harness](https://github.com/No-Scrolling/ink/blob/dfef70ed9f2d5564914c868ff332eaa98c288b83/test/performance/README.md) and retain raw paired samples. Compare each candidate with current production code, rather than an older unoptimised baseline. Include memory, allocation, APK size, startup and unchanged-path controls where relevant. Measure p95/p99 and throughput as well as medians.

Host scene readiness, Vulkan submission and display presentation are different endpoints. Emulator results do not establish physical Light Phone III performance or energy use. For later emulator experiments, use `emulator -avd Light_Phone_III -writable-system`. This research review changes no rendering code and reports no visual defect.

## Implementation follow-up

The first native source/view slice is implemented. `createSource`, equality-filtered views and `SourceList` now share Rust-owned records and immutable snapshots. The measured 5,000-record host edit fell from 0.572 ms to 0.131 ms; Android emulator input-up to final scene submission fell from 2.200 ms to 1.689 ms. Full replacements also improved. Android edit p95 increased from 3.458 ms to 3.668 ms. See `docs/research/native-source-results-2026-10-22.md` for scope, full results, controls and evidence. The other literature candidates remain research leads.
