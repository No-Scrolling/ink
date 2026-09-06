# Developing an Ink application

Run `ink dev --device <serial>` inside the app, or use `ink -C <app> dev`. The first successful build installs the development APK. Keep the command running: subsequent application edits compile JavaScript and assets, transfer a complete generation through ADB into app-private storage, then activate it without Gradle or an APK installation. `--once` builds and launches once. `--logs` streams application errors and logs.

Development builds use development React, readable JavaScript and source maps. Compatible component edits use React Refresh and preserve mounted component state. Changed hook signatures remount the affected component. Changing the persistent framework bootstrap, capabilities, Android resources, app configuration or native sources rebuilds the development host. Changing icon masks restarts the UI runtime. A runtime restart resets React state; persisted application data remains available.

The watcher follows the compiler manifest's resolved inputs, including linked modules outside the application directory. It also watches app files and framework native/compiler sources so new imports and configuration changes invalidate the graph. It excludes `.git`, `node_modules`, `.ink`, `target`, `build`, `.gradle` and `dist` from recursive scans; resolved dependency files are watched individually. Polling is debounced before compilation.

A failed compilation leaves the running application alone. Runtime errors appear in a development dialog with a Reload action and are written to Logcat; source-map locations are resolved to their original source when a mapping is available. Correct the source and save to recover through the same CLI session. A failed native build can also be corrected without restarting the CLI.

Background jobs copy the selected worker bundle into immutable, content-addressed app-private storage when they are scheduled. Retries keep that bundle, so activating a UI generation does not change code underneath an existing job. The newest three development generations are retained. Worker bundles and audio used by detached playback are pinned separately by content hash; uninstalling clears these caches.

Release builds retain minification and omit the native development evaluation command. Development activation is gated by Android's debug build constant and requires the debug APK's `run-as` access through ADB.

Validation of a development-loop change should include a stateful component edit, a hook-signature edit, a syntax error followed by correction, a runtime error followed by Reload, a linked-package edit, an asset edit and a native-requirement change. Full application workflows and physical-device measurements are separate checks.

## Verification recorded on 6 September 2026

A standalone counter on the Android emulator retained its count across a compatible label edit, reset component state after a hook-signature change, and updated an imported module outside the app directory. These edits kept the Android process running and used bundle transfer without Gradle or installation. An injected component error mapped to its exact original TSX line and column; correcting the source dismissed the error dialog and recovered in the same process. A fresh watcher installed once before waiting for edits. These are bounded development-loop checks, not physical-device or complete application workflow verification.
