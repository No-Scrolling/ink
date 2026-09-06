# Native startup memory reductions on LP3

6 September 2026. Final median idle PSS is **18.07 MiB for the counter and 25.80 MiB for the non-virtualised 1,000-row list**. The list meets the requested 32 MiB target. The counter remains 73 KiB above 18 MiB; none of its three final samples was below the target.

## Measurements

Physical LP3, Android 14, ARM64 release builds, QuickJS and separate first-use web packaging. Each stage compares the preceding build with one additional change using three fresh processes per fixture and build. Order alternates; samples are taken two seconds after launch returns and rejected if another package is foreground. These are idle PSS measurements, not peak memory or a full performance benchmark rerun. The list fixture is unchanged and mounts all 1,000 rows.

| Additional change | Counter control → changed (MiB) | List control → changed (MiB) |
| --- | ---: | ---: |
| Initialise HTTP engine on first request | 25.70 → 22.50 | 34.84 → 31.21 |
| Full allocator purge after startup GC | 22.50 → 21.28 | 31.19 → 28.78 |
| Create keyboard and glyph rasteriser on demand | 21.20 → 20.68 | 28.82 → 28.29 |
| Software Canvas for surrounding Android views | 20.73 → 18.19 | 28.42 → 25.83 |
| Retain refresh metadata only in debug builds | 18.24 → 18.07 | 25.88 → 25.80 |

Controls were remeasured at each stage, so neighbouring rows differ slightly. The small metadata result is close to normal variation. Against the earlier split-web medians of 25.88/35.58 MiB, the final build is approximately 30%/27% lower; the paired comparisons above provide better evidence for individual changes.

Final counter samples: 18,484 / 18,505 / 18,533 KiB. Final list samples: 26,464 / 26,418 / 26,338 KiB. [All raw samples and summary](native-memory-lp3-2026-09-06/summary.json), [APK hashes](native-memory-lp3-2026-09-06/apk-hashes.txt). Raw text files are grouped by stage beside the summary. The existing [measurement script](split-web-lp3-2026-09-06/measure.ts) uses `control` and `split` filenames; here `split` means the additional change for that stage.

## Changes and trade-offs

The HTTP adapter previously created Android's HttpEngine even when the app never made a request, loading Cronet native code. It now initialises on use and shuts down only if initialised. Apps that use networking will still pay that memory and initialisation cost.

The one-off startup collection now requests Android's `M_PURGE_ALL` instead of `M_PURGE`. The project already requires API 34, which supports this option. It examines more allocator memory and can take longer; its pause cost has not been benchmarked. Collection is not repeated on every frame.

Keyboard construction and the system glyph rasteriser are deferred until needed. Keyboard appearance and preferences are retained and applied when created. Release builds no longer retain the Java icon byte array for development refresh or parse its refresh manifest.

The Android application window now uses software Canvas for surrounding views. **Ink's SurfaceView still renders the scene through Vulkan.** This avoids the additional Android HWUI path. Keyboard entry and dismissal worked in the emulator, but camera surfaces and keyboard animation performance have not been comprehensively verified on this configuration.

## Verification and remaining opportunity

Both final release APKs built successfully. TypeScript checking, runtime formatting, Cargo checks for runtime/compiler, and diff whitespace checks passed during the work. No tests were added. The counter incremented and the list scrolled on LP3: [counter](native-memory-lp3-2026-09-06/counter-lp3.png), [list](native-memory-lp3-2026-09-06/scroll-lp3.png). A separate emulator smoke app exercised first-use URL/Headers/Blob/WebSocket access, HTTP fetch returning 200, and keyboard input/dismissal with software surrounding views: [log](native-memory-lp3-2026-09-06/keyboard-web-smoke.txt), [source](native-memory-lp3-2026-09-06/web-keyboard-smoke.tsx), [screenshot](native-memory-lp3-2026-09-06/keyboard-emulator.png). The smoke app was removed afterwards.

Temporary native diagnostics measured approximately 0.65 MiB of live QuickJS memory after startup GC. Native executable mappings, the Vulkan driver/compiler and allocator pages accounted for substantially more. Diagnostics were removed from production. A renderer-disabled diagnostic produced a blank app and is not a valid benchmark result. Further counter reductions should investigate retained renderer allocations and native code footprint while preserving rendering; more JavaScript GC is unlikely to deliver a large saving.

Published comparison figures in the root README have not been replaced by these targeted experiments.
