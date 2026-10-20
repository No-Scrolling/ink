---
title: "Recent performance research candidates"
description: "Recent primary research ranked by relevance, potential upside and production cost for Ink."
---

Prioritise an authoritative collection-delta API for large lists, retain dependency-aware layout as the larger architectural candidate, and compare cache refinements against the policy already shipped. The smallest experiments are cost-aware artwork admission and compiler coverage for SDK components. This ranking predates implementation; see the [measured follow-up](recent-performance-results.md) for the experiments and decisions. Adaptive artwork retention and cooperative yielding were selected for production; collection and route preparation were deferred.

This review inspected `1796236` on 20 October 2026. Recent means prioritising the preceding two years, from 20 October 2024, without excluding a stronger older reference. Exa discovery requested 175 result slots across 21 searches and five workstreams: incremental computation/layout, React/compiler optimisation, scheduling, caching/rendering, and startup/navigation. That count includes duplicate results and is not a count of distinct papers read. alphaXiv supplied discovery, full-text reading and PDF queries; primary proceedings, author manuscripts and source code were checked for the shortlisted mechanisms.

## Candidate ranking

The order reflects expected value for Ink: an observed code-path match, the amount of work the mechanism could remove, and implementation cost and uncertainty. High upside describes a promising workload, not a forecast percentage or a benefit for every app. Benefits overlap and must not be added together.

| Rank | Candidate | Relevance | Potential upside | Effort | Research status |
| --- | --- | --- | --- | --- | --- |
| 1 | Propagate explicit keyed collection deltas from producer to native list | Very high | High for sparse edits in large collections | Large | New 2026 papers strengthen an existing, unimplemented direction |
| 2 | Recompute dirty layout fields and retain unaffected scene fragments | Very high | High when a small change currently triggers general layout | Large | PLDI 2025; carry-over from the previous review |
| 3 | Adapt artwork retention to changing reuse patterns | High for artwork-heavy apps | Medium; fewer repeat uploads and decodes | Medium | Two new OSDI 2026 designs |
| 4 | Weight artwork retention by reconstruction cost | High for mixed bundled-image encodings | Medium when expensive decodes recur; otherwise low | Small–medium | ACSOS-C 2025 short paper |
| 5 | Reduce SDK component work and review state ownership | High architectural fit; remaining cost unprofiled | Low–medium beyond the existing compiler | Small–medium | ICSE 2026 |
| 6 | Schedule resumable background work around foreground input | High for busy JavaScript apps | High for input tails under load; little idle benefit | Medium | Existing direction, refined by a 2025 draft API; recent academic design is preliminary |
| 7 | Prepare likely first-visit route resources while idle | Moderate | Potentially high for an expensive cold route; unknown in Ink | Medium–large | IWQoS 2025 |

## 1. Propagate explicit keyed collection deltas

[Stateful Differential Operators for Incremental Computing](https://dl.acm.org/doi/10.1145/3776728), POPL 2026, specifies how an operator converts input changes into output changes while maintaining its own index or other cached state. It verifies relational and string operators, rather than an ordered UI-list implementation. [DeCo: A Core Calculus for Incremental Functional Programming with Generic Data Types](https://dl.acm.org/doi/10.1145/3798264), OOPSLA1 2026, extends cached incrementalisation to user-defined containers. Its evaluation is explicitly a proof of principle using an unoptimised interpreter, not evidence of a fast production library.

**Ink change.** Add an opt-in revisioned collection source with keyed insert, remove, move and update operations. Preserve changes through derived projections and native list application. In [native-list.ts](../../packages/ink/src/native-list.ts), `replaceInPlace()` still scans every item and copies the projected array on change. The fallback [ReactList](../../packages/ink/src/list.ts) also builds metadata for the collection. A delta needs to originate before the producer performs a full `map()` or array copy; native-only changes cannot remove that cost.

**Upside and limits.** A one-item edit could approach work proportional to the changed items and affected visible rows, rather than collection length. An index-dependent projection, global filter/sort change or order change can legitimately invalidate many rows. Maintain immutable React snapshots, stable keys, current-item callbacks and a full-snapshot recovery path for an unknown base revision. Do not expose mutable data through `useSyncExternalStore` or hide a full array materialisation in the new API.

**Decisive measurement.** Compare one-key edits, inserts and moves at 1,000, 10,000 and 100,000 items, then dense replacements. Time producer work through native scene application; record allocations, projected rows, bridge payload and memory. Verify final order, values and callbacks. The previous chunked-version trial left a 10,000-item edit at approximately 0.589 ms in both versions because it did not remove the JavaScript scan.

## 2. Recompute only affected layout fields

[Spineless Traversal for Layout Invalidation](https://dl.acm.org/doi/10.1145/3729322), PLDI 2025, visits dirty fields in dependency order using a priority queue and order-maintenance structure, avoiding tree traversal through unchanged auxiliary nodes. Its reported 1.80× mean improvement compares two generated incremental layout engines over web-page traces. It does not predict an Ink speed-up.

**Ink change.** General [relayout_scene_inner()](../../crates/ink-core/src/lib.rs) clears scene and hit-test collections and lays out the root. Retain layout outputs and scene fragments with explicit invalidation for text, dimensions, sibling flow, clipping and hit regions. Existing fixed-text patches already bypass some of this work, so target changes that miss those fast paths.

**Upside and limits.** This could remove most general-layout work for a local change. It requires a dependency model and correct propagation when geometry changes. The previous measurement cache reduced measured nodes from 1,551 to 1,051 but made the resize workload slower, 2.374 → 2.441 ms. That rejected cache was not this complete design.

**Decisive measurement.** Use local text/size changes inside deep and wide trees, with dense resize as a control. Measure total apply/layout/submission, visited fields and retained fragments. Check final geometry, clipping, hit targets and saved pixels before claiming a visual result. Begin with one bounded container type rather than replacing all layout at once.

## 3. Adapt artwork retention to changing reuse

[Merlin: An Efficient Adaptive Cache Eviction Algorithm via Fine-Grained Characterization](https://www.usenix.org/conference/osdi26/presentation/li-liujia), OSDI 2026, separates short-term hotness from long-term popularity using several FIFO/ghost queues and a sketch. It supports variable-sized objects. Its server-cache throughput results include multicore effects absent from Ink's renderer-owned cache.

[Learning-Augmented Heuristics](https://www.usenix.org/conference/osdi26/presentation/xia), OSDI 2026, introduces S4-FIFO: a simple request path with occasional asynchronous cache-level parameter selection. Its model was trained on production server traces; successful transfer to small mobile caches is unestablished. Treat Merlin and S4-FIFO as competing experiments for one opportunity.

**Ink change.** Compare these policies with the size-aware aggregate-victims admission already implemented in [ImageCache](../../crates/ink-renderer-vulkan/src/compact.rs). The shipped policy already reduced the artwork fixture from 48 to 23 repeat misses and 0.978 to 0.832 ms per 66-draw cycle. Adding TinyLFU or replacing LRU is therefore completed work.

**Upside and limits.** Changing hot sets, route revisits and repeated scans may leave further avoidable uploads. First distinguish demand from speculative preloads: both currently affect frequency and pinning. Extra queues and metadata can cost more than they save when only a few dozen textures fit. Preserve the 8 MiB retention target, mandatory current/preloaded textures, transient rendering and fence-safe resource lifetimes; that target is not a hard cap on texture memory.

**Decisive measurement.** Capture real artwork access traces and replay today's policy against Merlin at identical byte budgets. Include mixed sizes, scans just above capacity, shifting hot sets and discarded preloads. Inspect byte/object misses across a capacity sweep before integrating a winner. Then measure policy CPU, decode/upload work, p95 frame latency and texture plus metadata memory. Consider S4-FIFO only if fixed-policy tuning has meaningful headroom.

## 4. Include reconstruction cost in artwork admission

[Render Cost Driven Caching for Object-Based Media](https://eprints.lancs.ac.uk/id/eprint/233073/1/IEEE-conference-template-062824.pdf), an [ACSOS-C 2025 short paper](https://2025.acsos.org/details/acsos-2025-papers/24/Render-Cost-Driven-Caching-for-Object-Based-Media), shows that cost-informed partitions can reduce total reconstruction cost despite a higher miss rate. Its synthetic object-based-media experiments support the objective, not a ready-made image policy.

**Ink change.** Bundled JPEG/zlib texture misses reconstruct pixels inside `ImageCache::prepare()`. Current admission considers frequency and bytes, but not reconstruction cost. Compare a bounded estimate of avoided decode/upload work across a candidate and all its victims. Remote images already contain decoded RGBA at this seam, so retaining their texture does not necessarily avoid another network fetch or decode.

**Upside and limits.** This is a relatively small experiment if representative bundled assets have materially different miss costs. Avoid noisy per-frame timing decisions and indefinite retention of a formerly expensive image. It overlaps candidate 3; neither policy should be credited with the same avoided miss twice.

**Decisive measurement.** Use JPEG, zlib RGBA and already-decoded images with matched reuse, followed by real app traces. Measure total reconstruction time, uploads, interaction p95 and policy overhead at the same memory budget. The previous decoded-pixel fixture cannot establish this benefit. Accept lower total latency, rather than a better hit ratio alone.

## 5. Reduce SDK component work and review state ownership

[Remediating Superfluous Re-Rendering in React Applications](https://www.franktip.org/pubs/icse2026rerendering.pdf), ICSE 2026, identifies five patterns involving controlled inputs, state that does not affect UI, child memoisation and callback identity. It reports a 20.54% mean rendering-time reduction on 23 projects, but its static transformations are potentially unsound and one project changed behaviour. The authors could not complete a meaningful comparison with React Compiler. The result does not establish an advantage over Ink's current compiler.

**Ink change.** [bundle-javascript.js](../../crates/ink-compiler/src/bundle-javascript.js) already enables `reactCompiler: { target: "19" }` for app source. SDK modules outside the app root are outside that scope. Profile remaining SDK wrappers/hooks, then try compiler coverage or carefully reviewed state placement where work is redundant. Diagnose callback identity that defeats memoisation; preserve dependencies and component lifetimes.

**Upside and limits.** Likely smaller than the shipped custom-row compilation win. Native-compiled rows already bypass much of React, and simple host wrappers may be cheap. Do not automatically replace controlled inputs or state with refs: Ink's native editor acknowledgement and React update semantics are observable contracts.

**Decisive measurement.** Compare SDK-heavy forms, navigation and React-fallback rows with app compilation unchanged. Record component execution, QuickJS time, total acknowledgement latency, bundle size and memory. Include changing callbacks, effects and controlled input acknowledgements. Keep this only if total cost falls.

## 6. Schedule resumable background work around input

The [2025 Prioritized Task Scheduling draft](https://wicg.github.io/scheduling-apis/) supplies an implementable model for explicit task priorities, cancellable tasks and `yield()` continuations. It is a draft API specification, not a peer-reviewed performance paper. [Towards Hybrid Cooperative-Preemptive Scheduling](https://atlas.cs.brown.edu/pdf/cocoon:plos:2025.pdf), PLOS 2025, discusses controlled pre-emption, but describes an implementation in progress. It does not justify pre-empting arbitrary QuickJS execution.

**Ink change.** [ink-runtime](../../crates/ink-runtime/src/lib.rs) bounds pending-job drains by count, while Android bridge drains have time/count budgets. A single long synchronous JavaScript operation can still delay the React acknowledgement of input. Provide an opt-in chunk/continuation API for costly filtering, indexing or importing, with cancellation of superseded generations and foreground work scheduled at safe checkpoints.

**Upside and limits.** The goal is lower input p95/p99 under competing work; aggregate throughput may decline. Preserve synchronous callback and promise ordering. A cancellation interrupt is not a resumable continuation, and native editor/scroll responsiveness does not imply JavaScript acknowledgements are timely.

**Decisive measurement.** Use the existing prescribed-arrival conversation input workloads with realistic CPU-bound background transformations. Measure arrival-to-acknowledgement tails, dispatch lateness, completed background work and memory. Check ordering and cancellation. This refines the earlier cooperative-scheduling proposal rather than counting as a newly discovered academic win.

## 7. Prepare first-visit route resources while idle

[MAPP: Predictive UI View Pre-caching for Improving the Responsiveness of Mobile Apps](https://par.nsf.gov/servlets/purl/10598216), IWQoS 2025, predicts future Android views and performs view inflation early. Its 59.84% mean response-time reduction comes from its own Android API workload and user traces; the view hierarchy and construction process differ from Ink.

**Ink change.** [file-router.tsx](../../packages/ink/src/file-router.tsx) already keeps covered navigation entries in React `Activity`. Investigate first-visit resource preparation, rather than adding a general revisit cache. An explicit next-route hint could prepare immutable assets or pure descriptors within a bounded idle budget. Do not run arbitrary components early: that can trigger hooks, effects or application side effects.

**Upside and limits.** Potentially useful if first navigation has significant resource preparation cost. Wrong predictions consume CPU/memory and can displace useful artwork. Begin with deterministic hints and cancellation; a per-user prediction model is unjustified without evidence.

**Decisive measurement.** Profile the slowest first route, separating module evaluation, React construction, layout, decode, upload and presentation. Compare correct and incorrect hints while foreground input continues. Measure first-navigation p95, input delay, wasted work and memory. Defer if the relevant work is already hidden by preloads or existing native templates.

## Watch list and exclusions

- **QuickJS native ahead-of-time compilation.** [JWST](https://onlinelibrary.wiley.com/doi/10.1002/ett.70232), published August 2025, compiles QuickJS bytecode through LLVM while retaining interpreter support for dynamic code. This directly matches Ink's runtime family, but it is a substantial runtime/compiler fork. Its reported startup gain is against JavaScriptCore on React Native 0.68, not Ink's precompiled-bytecode path. The paper reports substantial code expansion. No maintained, reusable source and licence were verified in this search. Profile remaining bytecode evaluation before considering a feasibility spike.
- **Scan-resistant cache refinements.** [SR-Gadgets](https://arxiv.org/abs/2609.30468), first posted 24 September 2026, is a recent preprint addressing repeated scans and miss-ratio cliffs. Peer-reviewed publication was not verified. Its TinyLFU baseline differs from Ink's aggregate-victims policy; use it to design adversarial traces before considering a policy adaptation.
- **Atlas repacking.** [Real-Time Compact Atlases for Texture Space Shading](https://arxiv.org/abs/2502.17712), 2025, concerns 3D shading charts. It does not solve persistent glyph eviction, stable UVs or fence-safe retirement in Ink. The code seam is real, but this paper is a weak fit.
- **General partial redraw.** No sufficiently direct recent primary paper was found to rank it as a new literature-backed winner. Retained pixels and swapchain damage differ from the sparse mapped-buffer writes already rejected on measured latency.
- **Kernel/OS rendering schedulers, server hydration and per-object learned eviction.** These have deployment, architecture or overhead mismatches. They rank below changes inside Ink's existing app/runtime boundary.
- **Minato–Morreale ISOP.** The earlier [ISOP review](minato-morreale-isop.md) found no matching Boolean hot path. The new search did not change that conclusion.

## First experiments

1. Measure sparse edits with an explicit collection source through the complete producer-to-scene path. This has the strongest asymptotic opportunity and directly addresses the limitation of the rejected chunk-only trial.
2. Profile bundled-image miss costs and SDK component execution. These decide whether the two smaller experiments have enough work to remove.
3. Replay representative artwork traces against the current policy and Merlin before adding another policy to the renderer.
4. Start incremental layout with one bounded container after its dependency and retained-fragment contracts are defined.

Use the existing [performance harness](https://github.com/No-Scrolling/ink/blob/main/test/performance/README.md) for comparable baselines and correctness checks. For emulator work, use `emulator -avd Light_Phone_III -writable-system`. Report host/QuickJS, native apply/layout, Vulkan submission and display presentation as separate endpoints. Existing software-Vulkan and emulator results do not establish physical Light Phone III latency or energy use.
