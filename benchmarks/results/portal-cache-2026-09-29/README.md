# Send native portal data only when it changes

Camera, video and map portals now compare their Rust state before creating JSON. An unchanged query returns null; Kotlin reuses the decoded portal. An empty string still means removal, and a fresh engine always sends its initial state. Camera comparison includes appearance as well as controller, preview kind and geometry.

Adapter synchronisation still runs on every call. This matters because Android views and controller sessions may need reattachment even when the Rust description has not changed. Camera review readiness is also still queried independently. The cache is scoped to the engine/Activity, not shared between applications.

This avoids JSON construction, Java string allocation and Kotlin parsing for repeated portal state. It does not eliminate JNI queries, locks, adapter synchronisation or rendering. There is no measured end-to-end latency improvement in this report. QuickJS remains unchanged at `-O3`.

Keyboard context has not been cached: its adapter has pending-edit and frame-acknowledgement behaviour that must continue even when text is equal. This change is restricted to portal transport.

## Validation

Release experiment: `experiment-20260929-001559-8fd1a93e`; provenance and hashes are in [manifest.json](manifest.json).

- Counter release APK: **2,663,584 bytes**, 16 bytes above the previous build.
- Template native library: 2,769,432 bytes, up 848 bytes. Template DEX grew by 72 bytes and its APK by 16,384 bytes owing to alignment (the isolated template package name also differs). This is an allocation/work reduction with a small code-size cost.
- Ink and Beeper TypeScript checks passed. Existing native/compiler suite: 16 tests passed.
- Emulator: camera preview, map and video remained visible after Home → reopen. Saved screenshots were inspected with agent-tools OCR and visually: [camera](camera-resumed.png), [map](maps-resumed.png), [video](video-resumed.png). Video remained paused after backgrounding, as expected.
- Three rounds through maps → camera → video retained PID 4669 ([record](navigation.json)); the filtered Ink/InkRust/AndroidRuntime error log was empty.
- Returning to the Home screen removed the native portal ([screenshot](home-cleared.png)).

The template used temporary package `com.vandam.ink.template.portalcheck`, preserving the existing installed app. It was removed afterwards, the emulator reservation released, and the isolated build workspace cleaned. This pass did not run LP3 latency benchmarks or test camera capture.

## Beeper resolution repair

The app's Bun `file:` dependency installation contained per-file symlinks from an older Ink checkout. Its package exports had updated, but `node_modules/ink/src/native-list.ts` was missing. Resolving `ink/internal/list` produced the misleading “Cannot find package ink” message from `@ink/files/media`.

Running `bun install --force` in `~/Developer/beeper` restored the links. The previously failing development bundle then passed, as did `bun run check` and `ink build --debug` (18.5 MB development APK). No Beeper installation or app-data changes were needed. Refresh local dependencies this way when new files are added to a linked SDK package.
