# Develop an app

Run the development command from your app directory:

```sh
ink dev --device <serial>
```

Use `ink devices` to find your phone or emulator. To run from another directory, use `ink -C <app> dev`.

The first build installs and opens a development APK. Keep the command running while you edit. Ink sends JavaScript and asset updates through ADB without reinstalling the APK.

- Add `--once` to build and launch once.
- Add `--logs` to stream app logs and errors.

## Save and refresh

Compatible component edits preserve React state. Changing a component’s hooks remounts it. Changes to icon masks restart the UI runtime. Native code, capabilities, Android resources and app configuration rebuild the APK.

A runtime restart clears React state but keeps saved app data. The watcher includes imported files in linked packages outside your app directory.

## Fix errors

A build error leaves the current app running. Fix the source and save to try again.

Runtime errors appear in a development dialog with a **Reload** action and in Logcat. Source maps point to your original code when a mapping is available. You can fix JavaScript and native build errors without restarting the command.

## Background work

Each scheduled job keeps a copy of its worker bundle, so an app refresh does not change a running job’s code. Ink keeps the newest three development bundles. Worker bundles and detached audio assets have separate caches; uninstalling the app clears them.

Release builds use minified code and exclude the development reload command.

## Verification

[Emulator checks](verification-2026-09-06.md) cover state-preserving edits, hook changes, linked packages and recovery from errors. Real-app workflows and physical-device performance need separate checks.
