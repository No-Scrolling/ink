---
title: "Literature-backed performance ideas for Ink"
description: "Five production candidates grounded in published literature and Ink's current implementation."
---

Prioritise explicit list changes, byte-aware image-cache admission and incremental layout. Cooperative background work and wider native-template compilation are additional candidates when app workloads justify them. The smallest isolated experiment is image-cache admission; explicit list changes offer the clearest opportunity to remove work that grows with the entire dataset.

This brief records the research phase. Exa supplied discovery and primary-source retrieval; alphaXiv supplied literature discovery and full-text PDF reads. The review inspected Ink at `5a707a4` on 20 October 2026. No production implementation, benchmark or device run was performed during that phase. Ranking reflects the inspected code and implementation scope, rather than predicted percentage gains. See the subsequent [implementation results](../../test/performance/README.md#literature-follow-up-20-october-2026) for measured gains, costs and rejected trials.

## Candidate order

| Priority | Candidate | Work it could reduce | First implementation scope |
| --- | --- | --- | --- |
| 1 | Explicit keyed list changes with shared snapshots | Whole-array change discovery and copies after small edits | Medium; opt-in source and protocol work |
| 2 | Byte-aware image-cache admission | Repeated artwork decoding, uploads and eviction scans | Small to medium; renderer-local experiment |
| 3 | Dependency-aware incremental layout | General relayout after a local size-changing edit | Large; staged layout and scene work |
| 4 | Cooperative background computation | Input acknowledgement delay during heavy JavaScript work | Medium; opt-in SDK/runtime facility |
| 5 | Bounded native-template specialisation | React window materialisation for additional custom rows | Medium to large; compiler and native identity work |

These mechanisms address different costs. Their benefits cannot be added together.

## 1. Carry explicit list changes through the whole pipeline

**Literature.** [DBSP: Automatic Incremental View Maintenance for Rich Query Languages](https://www.vldb.org/pvldb/vol16/p1601-budiu.pdf), PVLDB 2023, describes maintaining results from input differences. [Incremental Computation with Names](https://www.alphaxiv.org/abs/1503.07792), OOPSLA 2015, shows how stable names preserve reusable computations across changes. [RRB Vector: A Practical General Purpose Immutable Sequence](https://www.cs.purdue.edu/homes/rompf/papers/stucki-icfp15.pdf), ICFP 2015, supplies a structurally shared sequence supporting efficient indexed updates, splitting and concatenation.

**Current gap.** Ink already projects only replaced rows and transports sparse native patches. However, `replaceInPlace()` in [native-list.ts](../../packages/ink/src/native-list.ts) scans every item and copies `previous.data` with `slice()` for a change. App code may already have copied its input array. In Rust, [native_list.rs](../../crates/ink-core/src/react/native_list.rs) uses `Arc::make_mut(&mut list.versions)` while rendered nodes share that version array, so sparse patches can still clone a dataset-sized array. Sending fewer bridge bytes does not remove all this work.

**First release.** Add an opt-in keyed collection source with immutable snapshots and authoritative revisioned changes. Start with replacement at an existing key; retain today's array path. The producer supplies the base revision, next revision and changed records. Consume those changes through projection, commit construction, native row data and version bookkeeping. Use chunked or persistent storage where snapshots would otherwise copy the whole sequence. RRB is an option, not a requirement to implement a complete collection library.

Reject a stale base and recover from a full snapshot. Preserve unique keys, captured-state invalidation, callback-to-current-item resolution and consistent snapshots across React renders. Avoid a hidden `toArray()` or full key-map rebuild on the new path. Treat insertion, removal and reordering as a later extension: the existing Fenwick height index already accelerates height updates, but does not make arbitrary sequence edits free.

After that foundation, a narrow keyed `map`/`filter` view can propagate record deltas rather than rerunning application-wide transforms. Sorted views need an ordered index and deterministic tie-breaking; DBSP's multiset model alone does not supply UI order. A changed global predicate can legitimately affect every row. Import the delta model rather than a full database engine.

**Decision evidence.** Compare single and batched replacements across collection sizes using the existing `list-edit-100/1000/10000` and standard-row workloads. Include producer update, snapshot allocation, projection, transport, Rust application and layout. Record items inspected, bytes copied, projected rows, allocations and p50/p95/p99 latency. The goal is to remove dataset-sized discovery/bookkeeping for small changes; total frame time still includes layout and rendering. Array workloads remain a compatibility baseline. Persistent structures can lose on small datasets because of indirection and allocation overhead.

## 2. Admit artwork to the cache according to reuse and byte cost

**Literature.** [Lightweight Robust Size Aware Cache Management](https://www.alphaxiv.org/abs/2105.08770), ACM Transactions on Storage 2022, extends TinyLFU admission to variable-sized objects. Its Aggregated Victims policy compares an incoming object's recent frequency with the combined frequencies of enough eviction victims to make room. [FIFO Queues are All You Need for Cache Eviction](https://jasony.me/publication/sosp23-s3fifo.pdf), SOSP 2023, offers S3-FIFO: a small probation queue, a main queue and bounded ghost history to filter one-use objects.

**Current gap.** [ImageCache::trim](../../crates/ink-renderer-vulkan/src/compact.rs) has an 8 MiB retention target. It repeatedly scans the map for the oldest unprotected entry; evicting several entries repeats that scan. Every image is admitted. A long artwork scan can displace useful images, and a later bundled-image miss can repeat JPEG decoding as well as texture upload. Visible and preloaded images are already protected. Same-sized animation frames already reuse their texture.

**First release.** Compare a byte-aware TinyLFU policy and a byte-budgeted S3-FIFO variant against the current policy on captured artwork requests. Keep policy metadata bounded. Count demand separately from redraws and preloads, so a 60 Hz animation does not manufacture popularity. Retain image identity across generation changes. Replace repeated oldest-entry searches with explicit policy structures and maintained byte accounting.

Preserve mandatory current-scene textures and fence-safe resource lifetimes. Rejecting long-term cache admission must never prevent the current image from rendering: it still needs transient storage. If all mandatory textures exceed the nominal budget, preserve the current working-set behaviour. For a large candidate, decide admission against the complete victim set before evicting anything.

**Decision evidence.** Existing renderer workloads cover first frames, animation and atlas recovery, but do not establish cache-pressure/revisit behaviour. A subsequent experiment needs real renderer artwork-scroll, back-scroll and route-revisit workloads exceeding the retention budget, with mixed image sizes and preloads. Measure repeat misses, decode time, texture allocations, eviction CPU, uploaded bytes, retained/pinned/metadata memory and frame p95. A higher hit ratio alone is insufficient. S3-FIFO explicitly has weak twice-accessed patterns where a revisit occurs after probation expiry; back-scroll may favour LRU. The published multi-threaded server throughput gains are not Ink estimates.

## 3. Recompute layout fields only when their dependencies change

**Literature.** [Spineless Traversal for Layout Invalidation](https://www.alphaxiv.org/abs/2411.10659), PLDI 2025, stores dirty layout fields in a priority queue ordered like a correct full layout. Order-maintenance labels support structural changes. Recomputing a field propagates invalidation only when its value changes, avoiding searches through clean ancestors and siblings. Its reported 1.80× geometric-mean result concerns a generated layout engine replaying 2,216 frames from 50 websites on a desktop CPU, not complete Ink frames.

**Current gap.** Ink already bypasses layout for eligible fixed-width, single-line text edits through `fixed_text_patches()` in [react.rs](../../crates/ink-core/src/react.rs). Other local updates can call `relayout_scene()`. In [lib.rs](../../crates/ink-core/src/lib.rs), `relayout_scene_inner()` clears scene and hit-test collections and lays out the root. Wrapped text and virtualised list heights are cached, but that does not provide general dependency-aware layout.

**First release.** Start with per-node measurement reuse for ordinary stacks and rows. Key results by stable identity, content/style/child revisions, constraints and font scale. Separate measurement changes, position changes and paint-only changes. Propagate size invalidation only while computed sizes change. Keep the existing full path for unsupported structures and broad constraint changes.

The later Spineless-style step is an ordered dirty-field work queue with explicit dependencies. Efficient insertion/removal ordering and stale-entry handling are necessary; merely adding a heap does not provide incremental layout. Retained per-node scene fragments and hit regions are also needed to avoid rebuilding those collections after measurements are reused. Stage those changes separately.

**Decision evidence.** Target size-changing text/row updates, nested stacks, variable-height lists and viewport changes. Record `nodes_measured`, `measure_ns`, `relayout_ns`, scene-construction work and total ready-scene latency. Keep `react-small` as an existing fast-path regression, not evidence of new work avoided. Width or font changes may invalidate much of the tree; sibling positions may legitimately move. Preserve scroll anchors, input geometry, clipping and hit testing. Before claiming rendering equivalence, compare saved output pixels in affected regions as required by the repository rules.

## 4. Make heavy application work explicitly cooperative

**Literature.** [RT.js: Practical Real-Time Scheduling for Web Applications](https://www.sra.uni-hannover.de/Publications/2019/dietrich_19_rtss.pdf), RTSS 2019, inserts explicit yield points into annotated computations and schedules resumable jobs. [SEDA: An Architecture for Well-Conditioned, Scalable Internet Services](https://people.eecs.berkeley.edu/~brewer/papers/SEDA-sosp.pdf), SOSP 2001, contributes bounded admission, queue separation and application-aware aggregation. These are mechanism references, not reasons to import a server thread-pool architecture.

**Current gap.** The runtime already has count and byte limits, latest-message coalescing and a 256-pending-job drain limit. The Android host already batches some work. However, one application callback can still monopolise QuickJS, and the runtime's count limit does not bound elapsed computation time. Native input can respond visually while its JavaScript acknowledgement waits.

**First release.** Provide an opt-in facility for chunked filtering, indexing or message preprocessing. A continuation yields between bounded chunks while its partial result remains private. Publish one completed immutable result. Cancel superseded generations. At safe task-admission checkpoints, serve foreground events before admitting another background continuation, while guaranteeing background progress through ageing or a minimum service share.

Preserve existing synchronous callbacks and JavaScript microtask semantics. Do not insert arbitrary yields into React, interrupt a microtask checkpoint or reorder causally related replies, input and submit actions. RT.js explicitly discusses weakened consistency from inserted yields. A priority queue cannot preempt a long callback, JSON parse or indivisible library call. This facility changes responsiveness for participating workloads; it does not reduce the total calculation automatically.

**Decision evidence.** Use `conversation-input-background` and `conversation-input-under-load` for acknowledgement p95/p99, dispatch lateness, queue age and background throughput. Add realistic cooperative computation to the fixture before measuring yield points. Require exact message delivery and ordering, cancellation correctness and background completion. Separate acknowledgement latency from physical input-to-display latency.

## 5. Specialise a bounded set of pure native row descriptions

**Literature.** Jones, Gomard and Sestoft's published book [Partial Evaluation and Automatic Program Generation](https://raspi.itu.dk/~sestoft/pebook/jonesgomardsestoft.pdf), 1993, §12.2, describes bounded static variation: specialise a finite set of known alternatives while retaining dynamic selection. This provides a precise basis for a limited compiler extension, rather than automatic compilation of arbitrary React semantics.

**Current gap.** [native-lists.js](../../crates/ink-compiler/src/native-lists.js) already separates fixed host structure from projected values. Conditional property values are supported; custom wrappers and conditional JSX child trees can retain React window materialisation. Rust explicitly prohibits changing a list template after mount, and reuses row node IDs by template position.

**First release.** Measure which actual app rows fall back because of simple wrappers. Statically inline only local, non-recursive, hook-free wrappers that return supported primitives. Keep callbacks and dynamic values on the existing projection path. Preserve current plain-data guards and React fallback.

For conditional child shapes, introduce explicit native branch/switch semantics with capped variants/nodes/bytes. Do not swap template strings or eagerly evaluate both branches. Local switch nodes avoid generating every combination of nested conditions. Keep JavaScript truthiness and short-circuit evaluation in JavaScript, and preserve branch-local identity, current-item events, focus, measurements and scroll anchors. Effectful or unproven components retain React.

**Decision evidence.** Compare newly supported real rows with their original React path: compiler coverage, JS window callbacks/commits during scroll, ready-scene p95, mounted-node bounds, branch-change behaviour, bytecode/APK size and compile time. Use the existing native/React row-scroll workloads as reference paths. Already compiled rows receive no direct benefit; code-size growth can offset runtime savings.

## What to defer

- **Replacing bitmap text with GPU glyph outlines.** [GPU-Centered Font Rendering Directly from Glyph Outlines](https://jcgt.org/published/0006/02/02/paper.pdf), JCGT 2017, addresses resolution-independent text. Its own comparison reports substantially slower rendering than cached bitmap text. Ink's known-size UI glyphs do not establish a need for that trade-off.
- **Applying a cache policy directly to the glyph atlas.** Ink's shelf allocator currently clears on exhaustion before growing. Selective eviction needs reclaimable storage, valid UV references, current-scene pinning and GPU-safe retirement first. The existing disjoint-screen atlas workloads cannot prove both working sets fit in the same capacity.
- **A new VM/JIT or general allocator.** Release bytecode, native scrolling/clocks and renderer retention already exist. Profiles must first show interpreter dispatch or allocation dominates. Reusable bounded scratch is a narrower experiment than replacing runtime memory management.
- **A complete incremental rendering VM.** [An Incremental Rendering VM](https://www.vrvis.at/publications/pdfs/PB-VRVis-2015-015.pdf), HPG 2015, is a useful reference for retained changes, but its JIT/state-trie architecture does not map directly to Ink's few ordered 2D draws. An independent small follow-up is coalesced sparse instance writes: Ink currently uploads the interval between the first and last changed instance. Measure write-call overhead and preparation scans before changing it.

## First experiments and release evidence

Start with a list-path work/allocation profile and an artwork-pressure trace. The former checks the strongest scaling opportunity; the latter supports the smallest isolated policy comparison. Proceed to incremental layout when general relayout is a meaningful share of the chosen app's latency. Prioritise cooperative work or compiler coverage when those workloads actually occur.

The existing suites are described in `test/performance/README.md` and `test/performance/render.rs`. Alternate baseline/candidate runs with identical fixtures and settings. Retain raw samples and inspect p95/p99 where relevant. Include memory, throughput, code size and correctness costs; reject results that shift work outside the measured interval. Host/headless and emulator measurements do not establish physical Light Phone III latency or energy use. Preserve the existing correctness checks and pixel comparisons.
