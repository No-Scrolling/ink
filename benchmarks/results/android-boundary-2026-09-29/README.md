# Android boundary cleanup — 29 September 2026

QuickJS remains at `-O3`. This pass removes unused Android integration paths and avoids unnecessary work across JNI; it does not change JavaScript semantics or move the engine outside the APK.

## Release size

ARM64 Counter, release builds without benchmark instrumentation, using the experiment tool's development signing key:

| Item | Before | After | Reduction |
| --- | ---: | ---: | ---: |
| APK | 2,692,824 B | **2,663,568 B** | **29,256 B (1.09%)** |
| Native library | 2,517,024 B | 2,487,768 B | 29,256 B |
| DEX | 106,268 B | 104,440 B | 1,828 B |

Entries are stored uncompressed; APK alignment means individual entry savings do not simply add up to the APK reduction. Counter is now **2.66 MB / 2.54 MiB**. The native library still accounts for 93.4% of it.

Before: `experiment-20260928-233457-5e93e73a`. Final: `experiment-20260929-000554-ae1ffbcd`. The manifests alongside this report record hashes and build provenance. These are incremental savings over the previous primitive capability changes.

## Changes

- Use existing application capabilities to gate keyboard, camera, video and map synchronisation in Kotlin and their native entry points. Counter no longer makes eight unused native queries each time these synchronisation paths run.
- Read keyboard active/numeric/action state in one native call. Apps with input now make two calls including context, instead of four.
- Skip input-event scanning when text input is absent. Viewport notifications remain enabled independently, so virtualised scrolling still works.
- Return null when there are no platform calls or JavaScript errors. This avoids allocating empty Java strings and parsing an empty JSON array; real calls and errors retain their existing handling.
- Rename `syncCameraPortal` to `syncNativePortals` to reflect its camera/video/map responsibility.

No measured frame-latency improvement is claimed here. The size reduction and removed calls/allocations are concrete; the existing host checks were run alongside builds and their timings are not suitable for a performance comparison.

## Compatibility evidence

- `bun run check` passed; existing native/compiler checks: **16 passed**.
- Existing headless list run `headless-20260929-000025-bc6fde6b`: all 14 checks passed, including bounded windows, custom React state, sparse edits, fresh callbacks, anchoring and reorder.
- Existing native-controls run `headless-20260929-000058-ce9d544d`: passed, including recycled controlled inputs with Japanese/emoji values.
- LP3: Counter launched and reached Count: 5 ([screenshot](counter.png)). This tested the initial boundary candidate. Final Counter native/DEX/JavaScript payload hashes are identical ([hashes](counter-payload-hashes.json)); the subsequent production edit affects only the camera source group, excluded from Counter.
- Emulator: a release scroll fixture without the text-input capability reached rows 74–77 with artwork ([screenshot](scroll-far.png)). This exercises viewport delivery independently of keyboard support.
- Template: text and numeric input, Search and Done submissions, multiline Return ([screenshot](multiline.png)), local video playback, camera preview and map rendering passed. Numeric Done evidence: [screenshot](numeric-done.png).

Testing exposed a camera shutdown race: leaving before the asynchronous camera lifecycle starts could try to destroy an INITIALIZED lifecycle. The close path now sends ON_DESTROY only after creation. The old failure is retained in [camera-crash.txt](camera-crash.txt). With the fix, six rapid camera↔map round trips retained PID 4024 ([navigation record](camera-navigation.json)); [the map rendered afterward](maps-final.png), and the filtered final error log was empty. Camera capture itself was not tested.

The template used temporary package `com.vandam.ink.template.boundarycheck` in an isolated source snapshot, preserving the user's installed template and working manifest. All temporary apps were removed and both device reservations released. Build workspaces were cleaned; manifests and source archives remain in agent-tool evidence.

## Further opportunities

The next plausible Android boundary saving is to avoid serialising unchanged portal/input context. That needs a revision or dirty-state contract which also handles surface recreation and lifecycle reattachment; blindly skipping equal state could prevent native views being restored. Measure how often payloads repeat before adding that bookkeeping.

QuickJS, Unicode support and release optimisation levels are unchanged. This pass establishes a smaller baseline, not proof that no further size or speed improvements are possible.
