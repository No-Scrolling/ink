# Lazy image resources and compact native nodes

6 September 2026. Final median idle PSS on LP3 is **17.84 MiB for the counter and 22.59 MiB for the non-virtualised 1,000-row list**. The paired list baseline is 25.55 MiB: a **2.96 MiB (11.6%) reduction**. The counter difference is only 84 KiB and should be treated as noise.

## Measurements

Physical Light Phone III, Android 14, ARM64 release APKs, React and QuickJS. Three fresh processes per fixture and variant, alternating order. Idle PSS is sampled two seconds after activity launch returns, with a foreground check. Thermal status was 0 before the final idle run. These are targeted Ink comparisons; Expo and Light SDK were not rerun.

| Final build | Counter idle PSS | List idle PSS |
| --- | ---: | ---: |
| Before these changes | 17.92 MiB | 25.55 MiB |
| Lazy image resources + compact nodes | 17.84 MiB | 22.59 MiB |

Final samples in KiB:

| Fixture | Baseline | Changed |
| --- | --- | --- |
| Counter | 18,293 / 18,401 / 18,348 | 18,253 / 18,327 / 18,264 |
| List | 26,161 / 26,197 / 26,147 | 23,187 / 22,976 / 23,130 |

The baseline also measured below 18 MiB in this session, versus 18.1 MiB in the earlier full comparison. This does not establish a meaningful new counter saving or guarantee that every run stays below 18 MiB. PSS is process memory apportioned by Android, not peak memory.

Earlier paired stages isolate the changes:

| Additional change | Counter baseline → changed | List baseline → changed |
| --- | ---: | ---: |
| Lazy image resources | 17.88 → 17.90 MiB | 25.52 → 25.36 MiB |
| Compact nodes, with lazy resources in both builds | 17.96 → 17.83 MiB | 25.57 → 22.57 MiB |

Compact nodes account for the clear list saving. Lazy image resources avoid allocations, but their PSS effect here is small. Do not add stage differences to the final comparison: each baseline was measured independently.

[Raw samples and summary](renderer-nodes-lp3-2026-09-06/summary.json), [final APK hashes](renderer-nodes-lp3-2026-09-06/apk-hashes.txt), [changed source hashes](renderer-nodes-lp3-2026-09-06/source-hashes.txt). The final raw dumps are in `renderer-nodes-lp3-2026-09-06/verified/`; earlier stages are in `lazy-images/` and `compact-nodes/`. The existing [idle measurement script](split-web-lp3-2026-09-06/measure.ts) uses `control` and `split` filenames; `split` means the changed build here. Final source is based on `01be2c3ea4796a3676d9fc99ef4ba680a4a880b2` with the two source changes identified by the hashes.

## Implementation

- The image shader pipeline is created when the scene first uses an image or requests a system glyph, including emoji. Image and system-glyph GPU instance buffers are allocated on their first non-empty upload. The identical font/image bind-group layout is shared. Existing texture caching and incremental buffer uploads are retained.
- Native React nodes store component kinds as enum values rather than individually allocated strings. Raw text keeps its string directly, and Text nodes retain parsed size, alignment and line-limit fields instead of JSON property maps. Other component properties keep their existing representation. Updates replace the typed properties, and nested text still flattens into the rendered text node.

Both behaviours are automatic. Apps need no configuration or API changes. The list fixture is unchanged and still mounts all 1,000 rows.

Image resources remain allocated after first use, so image-heavy apps still pay their cost. Pipeline creation moves to first use and can add work to that interaction; first-image latency has not been benchmarked. This change does not implement off-screen image culling, image downsampling or a smaller font atlas.

## Verification

Core and renderer Cargo checks, including the renderer's `perf` feature, passed. Both benchmark release APKs and the smoke app built successfully. No tests were added.

Three alternating rounds of the existing benchmark workloads also completed on the final APKs: 100 counter taps, 12 scrolling swipes, and separate five-second continuous drags. Foreground checks passed, runtime logs contained no reported errors, and thermal status stayed at 0 at both ends of the run.

| Median interaction metric | Baseline | Changed |
| --- | ---: | ---: |
| Counter CPU time, 100 taps | 1,460 ms | 1,420 ms |
| List CPU time, 12 swipes | 1,440 ms | 1,450 ms |
| Continuous-scroll p99 frame interval | 16 ms | 16 ms |
| Counter PSS after interaction | 18.65 MiB | 18.66 MiB |
| List PSS after interaction | 26.37 MiB | 23.25 MiB |

CPU differences are small; there is no clear performance change in this sample. Every continuous-scroll sample had a 16 ms p99 frame interval, and SurfaceFlinger reported zero dropped frames in all recorded workloads. Counter presentation intervals include the gaps between injected taps and are not an animation-performance measure. [Raw workload samples](renderer-nodes-lp3-2026-09-06/workloads.json), [counter after 100 taps](renderer-nodes-lp3-2026-09-06/final-counter.png), [scrolled list](renderer-nodes-lp3-2026-09-06/final-scroll.png).

Manual smoke checks on LP3 and the emulator exercised first-use images before emoji, emoji before images, repeated image/emoji removal and reintroduction, nested counter text updates, and changes to text size and alignment. Saved screenshots were also checked directly for the unchanged title and button pixels. All temporary renderer diagnostics were removed.

- [Image-first source](renderer-nodes-lp3-2026-09-06/smoke.tsx), [emoji-first source](renderer-nodes-lp3-2026-09-06/smoke-emoji-first.tsx). The image import used the repository's `examples/light-template/assets/images/wallsocket.jpg`.
- LP3: [first image](renderer-nodes-lp3-2026-09-06/verified-image-first-lp3.png), [first emoji](renderer-nodes-lp3-2026-09-06/verified-emoji-first-lp3.png), [after repeated cycles](renderer-nodes-lp3-2026-09-06/verified-cycled-lp3.png), [text styling update](renderer-nodes-lp3-2026-09-06/verified-text-update-lp3.png), [log](renderer-nodes-lp3-2026-09-06/verified-smoke-lp3.log).
- Emulator: [first image](renderer-nodes-lp3-2026-09-06/verified-image-first-emulator.png), [first emoji](renderer-nodes-lp3-2026-09-06/verified-emoji-first-emulator.png), [after repeated cycles](renderer-nodes-lp3-2026-09-06/verified-cycled-emulator.png), [text styling update](renderer-nodes-lp3-2026-09-06/verified-text-update-emulator.png), [log](renderer-nodes-lp3-2026-09-06/verified-smoke-emulator.log).

The temporary apps were removed from LP3 and the emulator afterwards. The root README's full-framework comparison remains the earlier single-session run; these follow-up measurements are recorded separately.
