# Optional native UI families

Counter's release APK is **2,718,784 bytes (2.72 MB)**, down **164,580 bytes (5.7%)** from the preceding 2,883,364-byte release. The native library falls from 2,706,640 to **2,542,968 bytes**. JavaScript compresses to 46,977 bytes. The speed-focused release profile remains unchanged: optimisation level 3, fat LTO and one codegen unit.

## Changes

- The compiler's existing capability manifest now selects native list expansion, message composition, player composition and bound views independently. Gradle forwards these requirements as Cargo features. Counter selects none of these optional families. Rust core consumers retain all four by default; Android opts into the app's selected set.
- Compile-time feature guards allow LTO to discard unused composition, deserialisation and update paths. Primitive layout/rendering support remains shared; this does not remove every piece of list/player-related code from Counter.
- Audio processing no longer implicitly enables core default UI features. A final Cargo dependency check confirmed that enabling microphone processing alone does not re-enable optional UI families; `cargo check -p ink-audio` passed. Counter does not include that optional dependency, so its measured APK is unaffected.
- Player preparation now belongs to the normal component create/update path, rather than list expansion. Playback can work without the native-list feature.
- Shared React imports and pure, unused template/registry construction allow the bundler to remove conversation/reorder/list setup when unused. The annotations cover internal template builders with allocation-only callbacks, plus built-in Symbol/WeakMap construction.
- Literal host calls contribute requirements. Dynamic host calls outside Ink and generated entry code conservatively retain optional UI and image/network/input capabilities. The public native-view module retains broader support for its dynamic descriptions. No app configuration or public component API changes are required.

Reverb and Podcasts select lists and playback, excluding unused message and view expansion. The full template selects all four. [Capability evidence](capabilities.json).

## Validation

- SDK, template, Reverb and Podcasts type checks passed. Both real apps also bundled successfully with the current compiler; no new Reverb/Podcasts APKs were installed.
- All 16 existing native transport/compiler regression checks passed.
- Six existing core tests passed with full features. The two playback tests also passed with only `ui-playing`; the four list tests passed with only `ui-lists`. Core compiled with no default features; the compiler checked successfully.
- [List regression harness](list-checks.json): compiled Text/Stack and Row, custom React row state, bounded windows, row callbacks, prepend/reverse, sparse edits, callback freshness, deletion, reorder actions and conversations passed.
- [Native bindings harness](view-checks.json): all 500 cells across 200 resize updates, thumb/content dragging, reverse, hide/show and count/scrollbar changes passed. Mean CPU input-to-scene time was 0.267 ms on the Mac. This excludes GPU/display work, is not a new LP3 latency result and is not a paired performance comparison.
- Physical LP3: Counter launched and advanced from [zero](counter-before.png) to [five](counter-after.png) after five taps. OCR confirmed Count: 5. The test app was uninstalled, settings restored and the reservation released; no benchmark packages remained.
- Emulator: [native lists](lists-before.png), [selection/scrolling](lists-scrolled.png), [reorder](reorder.png), [reordered rows](reorder-after.png) and [messages](messages.png) rendered successfully. Row 2 moved below Row 3; conversations opened at the final records. Japanese and emoji content remained visible. The fixture was uninstalled afterwards. Existing Counter/template installations were left untouched.
- The full template release built successfully. Its existing emulator installation was not overwritten, so this run does not claim a fresh full-template device walkthrough.

## Evidence and retention

- Release build: `experiment-20260928-231424-1b35c63b`; [manifest](build.json), [APK contents](apk-entries.json), [image checks](image-checks.json).
- Headless runs: `headless-20260928-231424-59d3be27` and `headless-20260928-231822-766081d8`.
- An initial conservative scanner also counted the compiler's generated app-entry call and retained all features; that build was cancelled. Generated entry calls are now excluded from dynamic-host inference, while their imported component modules still contribute requirements.
- An early headless run refused results because sources changed during its execution; only the completed stable-source reruns above are reported.
- Isolated build workspaces were cleaned after retaining APKs, source archives and reports.

The remaining engine still contains shared primitives, QuickJS, standard-library/runtime support, JNI and JSON conversions. No language features, Unicode coverage or diagnostics were removed. Further reductions should be based on measurements of these remaining shared paths rather than assuming they are unused.
