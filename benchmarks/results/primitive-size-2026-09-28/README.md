# Optional primitives, bridge allocation and startup cleanup

28 September 2026. QuickJS-ng remains at `-O3`; release Rust remains `opt-level=3`, fat LTO, one codegen unit and panic abort. Native code is still inside each APK.

## Changes

- Android passes existing text-input, camera, video and maps capabilities through to matching core features. Canvas has its own automatically detected `ui-canvas` capability. Disabled host kinds fail at creation; their construction/layout paths can be eliminated by LTO. Core defaults still include all primitives, and dynamic native views conservatively retain these paths.
- React props and binding values share a decoder which borrows QuickJS's encoded JSON instead of allocating a second Rust string. JSON behaviour, message-size limits and owned values crossing the runtime queue are preserved. Ordinary JNI string conversions remain: Java encoding and thread ownership make those copies meaningful; no second transport protocol was added.
- Remaining Ink component modules share one React import. Unused navigation contexts are marked pure so the bundler can remove their initialisation. Active navigation listeners and runtime dispatch remain intact.

## Release size

| Counter | Before | After | Saved |
| --- | ---: | ---: | ---: |
| APK | 2,718,784 B | 2,692,824 B | 25,960 B (0.95%) |
| Native library | 2,542,968 B | 2,517,024 B | 25,944 B |
| JavaScript, raw | 148,372 B | 148,238 B | 134 B |
| JavaScript, stored | 46,977 B | 46,958 B | 19 B |

APK totals include ZIP/signing/alignment effects, so constituent differences do not sum exactly. Counter still selects only `external`; template and list capabilities remain present. The full template APK is 33,467,511 bytes and the list fixture is 3,207,164 bytes. These are arm64 release builds signed with the local development key.

Baseline: `experiment-20260928-231424-1b35c63b`. Final: `experiment-20260928-233457-5e93e73a`. Final APK SHA-256: `9404d9418d7693928516deb1445193c4bc38f88fdebb585d126a08257ba947c2`. See `build.json`, `apk-entries.json` and `capabilities.json` for provenance.

## CPU comparison

Twelve alternating before/after rounds of 200 updates with 20 warm-ups; saved binaries, no concurrent builds. Each run checks all 500 labels, resize, thumb/content dragging, reverse, hide/show and count changes. See `paired-cpu.json`.

| Mean, milliseconds | Before | After |
| --- | ---: | ---: |
| JS and transport | 0.1596 | 0.1581 |
| Apply and layout | 0.1292 | 0.1278 |
| Input to scene | 0.2893 | 0.2864 |
| Mount and setup | 13.73 | 14.05 |

The roughly 1% update difference does not establish a speed improvement. Earlier rounds were around 0.27 ms for both builds. These are desktop CPU results, excluding Android delivery, rendering and display presentation. Startup also shows no demonstrated improvement; removal of unused initialisation is a code/bundle simplification.

Baseline harness: `headless-20260928-232603-476b01d7`. Final harness: `headless-20260928-233412-ea281923`.

## Compatibility

- Existing TypeScript checks, native/binary/list compiler regressions, core tests and runtime tests passed. Core also passes checks without default features and playback/list tests with only their respective features enabled.
- Headless list checks cover compiled rows, custom React state, sparse edits, current callbacks, bounded windows, reorder and conversations. Native control checks cover keyed edits and controlled Japanese/emoji input across recycling.
- LP3: Counter 0 → 5; template navigation, normal text input, native bound count/input, Japanese and emoji, local video and map rendering checked. Camera page renders its permission-denied state; camera preview/capture is not validated. No new Canvas visual fixture was added.
- Emulator: compiled list scrolling/selection and stateful React row callback (`row-3:1`) checked. Screenshots retain the evidence; OCR confirms headers that the inline image preview sometimes omits.
- Reverb and Podcasts production JS bundles compiled. Their installed apps were not replaced.
- Device screenshots were taken with the initial candidate (`experiment-20260928-232759-c52008c1`). The final decoder uses the same borrowed JSON as text rather than bytes to avoid redundant UTF-8 checking; final headless list/native-control runs passed and the final Counter APK again reached Count: 5 on LP3.

Our temporary Counter/template/list installations were removed and reservations released. A new LP3 template installation appeared at 23:34:48, after the first reservation had been released. The tool correctly refused to overwrite it for a second template check, and it was preserved. Existing emulator apps were also preserved. Isolated build workspaces are cleaned; source archives and APK hashes remain in agent-tools.
