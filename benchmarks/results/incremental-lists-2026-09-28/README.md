# Incremental lists and artwork look-ahead

**Artwork follow-up:** the look-ahead described below was removed. A device reproduction identified missing native-request dispatch during momentum scrolling. See the [confirmed reproduction and fix](../fling-images-2026-09-28/README.md). The list update optimisation remains.

The public List API is unchanged. Compiled lists retain native values for unchanged records, update JavaScript callbacks independently, and send keyed record changes instead of the complete dataset. Key order crosses the bridge only for insertion, deletion or reordering. Rust reuses stored records and no longer retains a second copy of the dataset/template in generic host properties.

Projection still visits every item when its dependencies change. This is a reduction in allocation and transport, not an O(1) JavaScript update algorithm. Arbitrary React rows retain the compatibility implementation.

## Mac CPU comparison

Three alternating baseline/candidate pairs, 1,000 records, five warm-ups and 50 measured operations per case. Values below average the three run means. Raw output is in `paired-list.json`.

| Operation | Before | After |
| --- | ---: | ---: |
| Text/Stack: unrelated parent state | 3.494 ms | 2.703 ms |
| Text/Stack: one-row edit | 3.545 ms | 3.201 ms |
| Standard Row: unrelated parent state | 11.245 ms | 6.351 ms |
| Standard Row: one-row edit | 11.304 ms | 6.902 ms |
| Text/Stack: native window scroll | 0.066 ms | 0.071 ms |
| Standard Row: native window scroll | 0.085 ms | 0.094 ms |

The standard Row cases improved by approximately 44% and 39%. Window scrolling became slightly more expensive in this run; the candidate includes artwork look-ahead. These are host CPU measurements, not LP3 frame latency or a controlled attribution of each individual change.

Baseline: `headless-20260928-170755-7fa2e940`. Candidate: `headless-20260928-171925-7a1731fb`. The candidate adds assertions for zero records on unrelated updates, one record per edit, and a synthetic artwork prefetch check. Hyperfine averaged 1.209 seconds for five complete candidate runs. The former 0.389-second suite measured less work and is not comparable to this expanded suite.

Checks cover fresh callback state, single-row edits, deletion, prepend, reverse anchoring, bounded windows, custom React state and indirect React text. Compiler checks (12), core checks (4), Ink and Reverb TypeScript checks pass. Reverb's development JavaScript bundle builds successfully.

## Bridge experiment

A separate direct primitive-binding conversion experiment averaged 0.2653 → 0.2604 ms for 500 bindings plus resize across three alternating pairs of 1,000 updates using identical fixture assets (`paired-bindings.json`). JavaScript/transport averaged 0.1492 → 0.1450 ms. This small gain did not justify retaining the extra conversion path and its semantic surface: the experiment was removed. Complex properties and binding values still use the established JSON conversion.

The candidate benchmark/APK above includes that subsequently removed experiment. The List workload uses ordinary property updates, not native binding value operations. Its sparse-list and artwork implementations remain unchanged. Final-source headless verification: `headless-20260928-172541-347d334a`.

## Reverb artwork

Native lists now speculatively decode small local `ink-file://` artwork beyond the mounted window: up to 16 following and eight preceding rows, at most 24 images, with a 2 MiB estimated RGBA budget per list. Existing decoded-image cache limits remain in force. Prefetch does not mount additional controls or add images to the initial-screen blocking set. It does not fetch remote artwork.

The headless fixture confirms requests beyond drawn rows, bounded request counts and ready covers after a two-screen jump. It supplies synthetic decode completions; it does not establish real device loading latency.

An isolated Reverb APK was exercised on the emulator with 48 temporary albums and on LP3. The emulator albums and app were removed afterwards. Short scroll probes completed (`probe-20260928-172133-ef23e3d2`, `probe-20260928-172230-8491f0f7`). Successive stopped-scroll cover crops were identical on both devices (`image-20260928-172121-f07822b4`, `image-20260928-172230-7e017c0d`). **Settled screenshots do not prove that cold scrolling has no blank-cover frames. User visual verification remains pending.**

There are two cold paths: Ink decoding an existing cover, and Reverb extracting/publishing artwork during a first library scan. Reverb's `MusicLibrary.kt` publishes partial artwork every 500 ms and constructs album mosaics afterwards. Native prefetch cannot decode artwork whose URI the app has not supplied yet. No change to Reverb's extraction policy was made.

The isolated LP3 app `com.vandam.benchmark.reverb` was deliberately left open for user verification; the device reservation was released. The normal Reverb app and its data were not replaced. Source APKs: `experiment-20260928-171849-4367d4e7`; build workspace removed. Local cold-scroll recording and captures remain in `.agent-tools/reverb-image-investigation/`.

Reverb's original build failure was a stale local `file:` dependency installation: its updated Ink exports referred to new files absent from the per-file symlinks in `node_modules/ink`. `bun install --ignore-scripts` refreshed those links. Both development bundling and TypeScript checking then passed.
