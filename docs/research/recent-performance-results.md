---
title: "Measured recent performance experiments"
description: "Before-and-after results for seven research-informed Ink performance candidates."
---

Adaptive artwork retention and cooperative JavaScript yielding were selected for production. Collection and route-preparation prototypes were measured, then deferred. Bounded layout retention, reconstruction-cost weighting and SDK-wide React compilation were tried and removed.

The baseline is `1796236`. The measurements below use saved builds of the earlier four-change candidate with identical workloads and fixtures. That candidate also included the deferred prototypes; its combined startup, APK-size and emulator-control results do not describe the smaller production selection. Original source snapshots and raw samples remain available.

## Selected changes

The artwork policy applies automatically. Apps must split expensive work into chunks and call `yieldToInput()` to use cooperative yielding.

### Adaptive artwork retention

The policy adapts Merlin's released FIFO/ghost design to Ink: filter, staging and core queues; an adaptive frequency guard; a 4 KiB sketch; count-based ageing; and at most 1,024 ghost records, also limited by represented bytes. Images larger than the filter budget enter core, while images larger than the whole 8 MiB target remain transient. GPU resource protection and retirement remain separate from policy membership. These are explicit mobile-cache adaptations, not an exact reproduction of the paper or its server evaluation.

| Renderer workload | Before CPU, ms/cycle | After CPU, ms/cycle | Repeat misses |
| --- | ---: | ---: | ---: |
| Changing hot set: 84 draws | 1.195 | 0.966 | 60 → 32 |
| Mixed encodings: 44 draws | 1.144 | 0.613 | 44 → 18 |
| Revisit: 66 draws | 0.788 | 0.762 | 23 → 21 |
| Large covers: 24 draws | 0.195 | 0.182 | 0 → 0 |

Changing-hot-set uploads fell from 15,729,984 to 8,389,952 bytes per cycle. Mixed-encoding uploads fell from 11,535,040 to 4,719,296 bytes. Every new artwork draw checks its output colour. These are CPU submission measurements using llvmpipe software Vulkan, excluding GPU completion and readback. JPEG decoding uses the host decoder; Android uses its platform decoder. The target is a retention budget, not a hard limit on visible or preloaded textures or whole-process memory.

### Cooperative computation

`yieldToInput(signal?)` resumes through an existing zero-delay timer. Apps must split their work into small chunks; arbitrary JavaScript is not pre-empted. The benchmark checks an independent checksum, completion and preservation of the latest native editor draft.

Across eight 16-million-item jobs per version, input acknowledgement p95 was 111.310 → 4.491 ms; the median was 29.858 → 0.901 ms. Each job supplied 100 measured inputs after 20 warm-ups, at prescribed 2 ms intervals. The 800 observations are correlated within their eight jobs. Job duration was 151.449 → 170.592 ms at the nearest-rank median. Yielding improves responsiveness at a throughput cost.

## Deferred prototypes

The collection source and route preparation are absent from the public SDK and current benchmark suite. These sections retain the prototype results for future work.

### Collection updates

The `collection(items, key)` prototype published immutable snapshots and bounded change history. A sparse `set()` copied one persistent-tree leaf and its ancestors. Eligible native lists projected changed items and sent the existing native patch. The native version array still copied; the entire pipeline was not logarithmic. Insertions, removals, moves, expired history and structural projection used a full fallback.

| Headless workload | Before, ms | After, ms | Reduction |
| --- | ---: | ---: | ---: |
| One edit among 1,000 items | 0.227 | 0.187 | 17.5% |
| One edit among 10,000 items | 0.608 | 0.176 | 71.1% |

The 10,000-item p95 was 1.985 → 0.207 ms. Untimed probes check insert/remove/move behaviour, old snapshot values, a history gap, all keys and a tap returning the latest item. An initial shared lookup helper regressed ordinary arrays; the final scan uses direct array access. Ordinary 10,000-item array edits measured 0.607 → 0.568 ms. These controls do not use the collection API.

### Route preparation

In the prototype, file pages could export `prepare(params, { signal })`. `prefetchRoute(destination, { signal })` loaded their data without mounting components. The router bounded active jobs to four, shared matching unsignalled in-flight requests and aborted its signals on unmount. Cancellation remained cooperative.

For a fresh first visit to a 2,000-track route, navigation measured 15.927 → 14.912 ms. Preparation ran before the navigation timer and took 0.621 ms at its JavaScript median, compared with a 0.005 ms baseline no-op. The navigation gain excludes preparation and is not evidence of the same reduction in total work. Ordinary navigation measured 1.001 → 1.010 ms.

## Emulator results and costs

These measurements compare the earlier four-change candidate with the baseline. Collection results and combined costs remain prototype evidence.

Release APKs ran on the Light_Phone_III AVD, launched with `emulator -avd Light_Phone_III -writable-system`. Timed blocks ran after/before/before/after. Collection and geometry retain 48 edits per version after six warm-ups per block. CPU input retains 16 independent jobs per version after two warm-ups per block. Each CPU job computes eight million items; the edit must arrive while it is running and the checksum must match.

| Emulator workload | Before median, ms | After median, ms | Before p95, ms | After p95, ms |
| --- | ---: | ---: | ---: | ---: |
| One edit among 5,000 rows | 10.465 | 7.410 | 16.088 | 19.308 |
| Input during CPU work | 774.246 | 14.129 | 880.649 | 33.436 |
| Geometry control | 15.733 | 16.185 | 19.340 | 27.002 |
| CPU job completion | 1169.000 | 1239.000 | 1281.000 | 1503.000 |
| Process start to content submission | 989.000 | 1036.000 | 1167.000 | 1323.000 |

Edit timing starts at Android input-up and ends at the first Vulkan submission after the resulting React commit. One CPU attempt had its edit arrive after the job ended; that attempt and the earlier partial CPU section remain in the logs but are excluded from the final pool. The CPU section was restarted, retaining every valid response regardless of latency. Startup retains 20 launches per version after two warm-ups per block, with filesystem caches retained; it ends at the first submission following the fixture's JavaScript content marker. Neither endpoint measures display presentation or physical Light Phone III performance.

The fixture APK grew from 3,548,630 to 3,558,546 bytes (9,916 bytes). Its compatibility fixture imports the SDK namespace to detect APIs in both versions; this is a fixture-specific size comparison. Headless runtime-to-library startup measured 11.086 → 11.473 ms. Full resizing measured 2.176 → 2.212 ms. The candidate did not improve every workload.

## Removed trials

| Candidate | Measured result | Decision |
| --- | --- | --- |
| Bounded retained layout | Targeted geometry edit: 1.381 → 1.334 ms; full resize: 2.234 → 2.296 ms | Removed. The small local gain did not justify the resize cost and retained state. This did not implement a general Spineless layout engine. |
| Reconstruction-cost weighting | Cost-only mixed-encoding cycle: 1.126 → 0.499 ms, 44 → 12 misses; changing hot set retained 60 misses | Removed. The combination with the adaptive policy produced 22 mixed-case misses, compared with 18 for the retained policy. The measured frequency × decode-cost score was an Ink adaptation, not the paper's cost-bin algorithm. |
| SDK-wide React Compiler | Ordinary navigation: 1.002 → 1.108 ms; runtime startup: 8.386 → 8.688 ms | Removed. Compilation passed, but this trial was slower. State ownership beyond the collection subscription remains application-specific. |

The first Merlin trial retained the released artifact's 10% maximum-object rule. It caused 24 repeat misses instead of zero on the six-large-cover control, increasing CPU time from about 0.195 to 0.800 ms per cycle. That default was removed before the final large-cover comparison.

## Validation and evidence

The host comparison uses three alternating rounds, 100 samples and 20 warm-ups per workload per process. All 34 workload contracts and idle checks passed in both versions. The renderer comparison uses five alternating rounds, eight measured cycles and two warm-ups, covering all 12 workloads. Renderer values above are the median of five process-reported medians; host and emulator values pool raw samples using nearest-rank quantiles. Builds and other checks did not run concurrently with timing comparisons.

All 143 existing unit, runtime, scene, SDK and compiler checks passed, as did the SDK type check and documentation validation. Fourteen regions across six existing visual states matched a saved baseline with the same fixture hash. The approved-reference suite stopped at a stale fixture-approval hash; reference and approval files were not changed. Six additional full-frame before/after comparisons of collection, geometry and CPU fixture states were pixel-identical. No new unit tests were added.

Local evidence is retained under `.test-output/recent-experiments/2026-10-20/`. `final-manifest.json` records source and binary hashes. `retained-v2/` contains final host samples, `retained-v2-cpu/` the eight-job comparison, `retained-render/` renderer samples, and `device-*.log`, `device-raw.json` and `device-pixels.json` the emulator measurements and saved-pixel comparisons. Earlier directories preserve removed trials. `results.json` aggregates the displayed values; `charts.html` is the self-contained chart source. Consult the [research candidate list](recent-performance-candidates.md) for primary literature, publication status and limits on transfer to Ink.

## Production selection validation

After removing the collection and route-preparation prototypes, all 143 existing host checks, SDK type checking and documentation validation passed. The smaller build passed all 30 current host workload contracts with 100 samples and 20 warm-ups, all 12 renderer workloads with eight samples and two warm-ups, and the idle check. The artwork workloads retained 32 changing-hot-set misses, 18 mixed-encoding misses and zero large-cover misses.

Fresh Android captures passed the existing pixel tolerances in all 14 regions across six states against the saved baseline with the same fixture hash. Reference approvals remain unchanged. These verification runs do not replace the earlier timing comparisons. Selected-build evidence and source snapshots use the `selection` prefix in the evidence directory; the deferred code is preserved in `before-cache-yield-selection-source.json`.
