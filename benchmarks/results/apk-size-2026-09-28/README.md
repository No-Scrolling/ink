# Counter release APK size

The original measured LP3 release APK is **3,242,820 bytes (3.24 MB)**. The shared React import and lazy link-detection fixes are now implemented. A fresh release is **2,883,364 bytes**, saving **359,456 bytes (11.1%)**, without changing the counter source or moving code outside the APK. The analysis below records the preceding isolated experiments; validation of the implemented changes is recorded at the end.

## APK contents

These are stored ZIP-entry sizes, including compression where present. KB and MB are decimal.

| Content | Stored bytes | Share of APK |
| --- | ---: | ---: |
| Native engine | 2,719,312 | 83.9% |
| Android/Kotlin bytecode | 403,848 | 12.5% |
| Application JavaScript | 51,440 | 1.6% |
| OkHttp public-suffix database | 42,484 | 1.3% |
| Other entries, ZIP/signing/alignment overhead | 25,736 | 0.8% |

The native library and DEX are stored uncompressed. The JS source is 162,653 bytes before compression. Percentages are rounded.

## Confirmed unused capabilities

Counter imports the main Ink entry point. `PlayingScreen` is unused, but its direct CommonJS React import leaves a 16-byte interop wrapper in Bun's output. The capability detector treats any output contribution from `playing.ts` as requiring both image and network support. That brings in Android networking code and the public-suffix database even though this app has no images or network calls.

The isolated prototype routes that import through Ink's existing shared React module, adding `useRef` to its exports. The playback module then contributes no output and those two capabilities disappear. The external-link capability remains.

| Stored size | Measured release | Shared React import | Also lazy link detection |
| --- | ---: | ---: | ---: |
| APK | 3,242,820 | 2,887,448 | 2,883,364 |
| Native engine | 2,719,312 | 2,706,640 | 2,706,640 |
| DEX | 403,848 | 106,268 | 106,268 |
| JavaScript | 51,440 | 51,440 | 47,354 |
| Public-suffix database | 42,484 | 0 | 0 |

The second prototype initialises `LinkifyIt` inside `textLinks` on first use. Its current top-level construction keeps linkify-it and Unicode matching data in Counter despite conversation components being unused. The diagnostic bundle attributed roughly 14 KB of source to these dependencies; the release experiment removes 13,382 JS bytes before compression. A full app must still retain link detection when it uses messages or links.

Both experiments use the archived release source, normal release optimisation and development signing. [The prototype patch](prototype.patch) contains all three changed files. Repeated signed ZIP overhead can vary slightly; the entry-level measurements identify the actual code/data removed.

## Native engine

The packaged ELF contains 2,077,632 bytes of machine code, 327,924 bytes of read-only data, 223,376 bytes of unwind records/index, and 61,072 bytes of relocatable read-only data. See [all sections](elf-sections.txt). Section sizes are not directly additive to file size because some sections are not file-backed.

Approximate machine-code ownership:

| Group | KB |
| --- | ---: |
| QuickJS C and compiler helpers | 800.5 |
| Rust standard library, core, allocation and backtrace/unwinding | 413.0 |
| Ink core UI/layout | 287.2 |
| JNI bindings/entry points | 169.8 |
| Other Rust code | 92.4 |
| JSON/Serde | 82.6 |
| Ink runtime/bridge | 66.8 |
| Ink Vulkan renderer | 65.4 |
| Font/Unicode code | 63.2 |
| QuickJS Rust bindings | 37.3 |

These are symbol groups, not independently removable dependencies. LTO attributes inlined code to its caller. The symbol-bearing rebuild uses the same archived source, NDK, features and release optimisation, but changing `strip` to `none` produced 360 extra machine-code bytes. Its `.text` is **not byte-identical** to the packaged binary, so ownership figures are approximate. APK and ELF section measurements come directly from the actual benchmark APK.

Large individual functions in that rebuild include QuickJS `JS_CallInternal` (45.5 KB), React host-node rendering (31.1 KB), layout dispatch (28.3 KB), native-list expansion (24.2 KB), native message rendering (20.1 KB), frame rendering (19.5 KB), and native-view expansion (14.6 KB).

## Priorities

1. **Fix capability retention first.** The shared React import experiment saves 355 KB with no need to weaken QuickJS or native UI support. Validate a plain counter, image/list apps and PlayingScreen so capabilities are removed only when unused. Avoid a minimum-byte heuristic: a small real call can still require a capability.
2. **Remove eager module initialisation.** Lazy link detection saves another 4 KB compressed and avoids unnecessary startup construction. Check other component-level registries and templates before treating their initialisation as a pure operation.
3. **Investigate app-specific native UI features.** Rust now correctly owns routine visual updates, but Counter still links generic dispatch for list, playback, message and view components. Capability-driven native feature selection could remove unused families while keeping the public API. Dynamic/custom rows and native bindings need conservative detection. No saving is quantified yet; function sizes are not additive removal estimates.
4. **Reduce JNI and JSON specialisation where measurement supports it.** These groups are meaningful but smaller than QuickJS and std. JSON remains used for platform messages and complex props; direct bindings did not make the entire parser obsolete. Keep error handling and conversion semantics.
5. **Treat interpreter/profile changes as a separate trade-off.** QuickJS remains necessary for app logic and React compatibility. Counter's diagnostic JS bundle is dominated by the 117 KB reconciler, not app code. Native ownership alone does not make React or the interpreter removable. Size-oriented optimisation needs paired CPU benchmarks; the earlier custom standard-library/profile changes were explicitly reverted and are not recommended as an automatic next step.

Keep Japanese/emoji support, runtime diagnostics and native library memory mapping. Removing these is a capability or diagnostic trade-off, rather than eliminating unused app features.

## Evidence

- [APK variants, hashes and entry sizes](profile.json)
- [Native symbols and ownership groups](native-symbol-profile.json)
- [Prototype changes](prototype.patch)
- Baseline build: `experiment-20260928-223409-efd7b183`
- Local analysis and prototype APKs: `.agent-tools/apk-profile-2026-09-28/`
- [Physical-device benchmark for the original release](../matching-counter-lp3-2026-09-28.md)

No phone changes were made during this analysis. Prototype workspaces were removed after retaining the patch and measurements.

## Implemented build validation

Final build: `experiment-20260928-230242-f638207b`. Counter is **2,883,364 bytes** and declares only the external-link capability. The full template release also builds, retaining its image, network, audio, video and other required capabilities. See the [build manifest](validation/build.json) and [capability lists](validation/capabilities.json).

Template validation exposed a separate missing `callNativeBytes` export in the split web runtime. The compiler now exposes that existing bridge function alongside `callNative`, allowing Blob handling to bundle correctly.

SDK/template type checks, all 16 existing native/compiler regression checks and `git diff --check` passed. These are build and compatibility checks, not a new performance comparison; the README's runtime table still describes the previously measured release.

The final Counter APK was launched on the physical LP3 and advanced from Count: 0 to Count: 5 after five taps. Saved-image comparison confined changes to the count, and OCR confirmed both values. [Before](validation/counter-before.png), [after](validation/counter-after.png), and [inspection evidence](validation/device-check.json). The app was uninstalled afterwards; no benchmark packages remained, and the reservation was released. This is a smoke check, not a repeated latency benchmark.
