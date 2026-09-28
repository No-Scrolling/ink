# Size opportunities with speed optimisation retained

Re-profiled the **2,883,364-byte Counter release** from `experiment-20260928-230242-f638207b`. No source or optimisation settings were changed during this analysis.

The release remains `opt-level = 3`, fat LTO, one codegen unit, aborting panics and stripped symbols. QuickJS's C build uses Cargo's optimisation level through `cc::Build`; no size-oriented C override is configured for Android. The macOS-specific compiler flag is unrelated to this Android APK.

## Packaged size

| Content | Stored bytes | APK share |
| --- | ---: | ---: |
| Native engine | 2,706,640 | 93.9% |
| Android/Kotlin DEX | 106,268 | 3.7% |
| JavaScript | 47,354 | 1.6% |
| Other entries and APK overhead | 23,102 | 0.8% |

Native machine code accounts for 2,068,176 bytes. Read-only data is 327,348 bytes; unwind records/index are 222,096 bytes; relocatable read-only data is 60,928 bytes. [Exact packaged ELF sections](elf-sections.txt).

## Approximate machine-code ownership

| Group | KB |
| --- | ---: |
| QuickJS C and compiler helpers | 800.5 |
| Rust standard library/core/allocation/backtrace | 411.7 |
| Ink core | 282.2 |
| JNI bindings and entry points | 167.6 |
| Other Rust | 92.4 |
| JSON/Serde | 82.6 |
| Ink runtime and bridge | 66.7 |
| Ink Vulkan renderer | 64.6 |
| Fonts and Unicode | 63.2 |
| QuickJS Rust bindings | 37.3 |

QuickJS C code is about 27.8% of the APK, not the entire native engine. Its tables and other read-only data are outside this machine-code table. The Rust runtime group includes ordinary collections and allocation, not just diagnostics; symbols explicitly labelled as Rust backtrace/symbolication/unwinding total about 154 KB, not the full 412 KB.

Attribution uses a symbol-bearing rebuild of the archived source with no optional native features, matching Counter's build. Changing stripping produces 360 extra `.text` bytes, and its `.text` is not byte-identical to the packaged binary. Group and function measurements are therefore approximate; LTO moves inlined work between symbols. [Symbols and verification hashes](profile.json).

## Recommended next work

1. **Select native UI families per app.** Generic host-kind dispatch currently references every native component family. Identifiable functions still linked into Counter include native lists (35,976 bytes), messages (21,436), playback (12,652) and native views (18,376). None is used by the counter fixture. These are investigation targets, not additive savings estimates: shared code, inlining, layout branches and metadata affect the final result. Generate capability requirements from surviving component/binding code and gate implementation families at compile time. Preserve conservative inclusion for dynamic custom rows and host-node names. Keep ordinary `ink` imports and full speed optimisation for included components.
2. **Audit eager JS component setup.** Reorder and conversation modules still construct native templates at module scope, and list support creates a registry. Moving unused setup out of startup is worth checking, though compressed JS is now only 47 KB, so this is a smaller size opportunity. Do not mark callbacks pure without checking side effects.
3. **Consolidate duplicated JNI/Serde specialisations where profiling supports it.** Share cold conversion/error paths while preserving typed hot paths. Native bindings still transport complex props and platform messages through JSON, so removing JSON wholesale would change behaviour.
4. **Keep QuickJS fast and compatible.** Retain `-O3` and full JS semantics. A smaller custom intrinsic set, removal of the parser, or dropping React would restrict supported applications; the current runtime evaluates source for app/bootstrap code. Profile-guided optimisation could be a later speed experiment, using representative startup, lists, text updates and audio/network events. No PGO gain is claimed here.

Do not reintroduce the reverted size-oriented profile or custom standard-library rebuild. Do not remove Unicode/emoji coverage or unwind diagnostics merely to lower the number. There is no measured basis here for promising a sub-2 MB APK under these constraints.

The first step should be app-specific native component selection, validated with Counter plus the full template and real apps. It can remove code Counter never uses while keeping the same optimised implementation in apps that need it. No new performance claims or phone tests were made in this analysis.
