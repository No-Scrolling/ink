# Android CLI workflows for Ink

Research date: 29 August 2026.

## Recommended interface

Keep the public surface small:

- `ink build [--debug]` lets Gradle assemble and sign the selected variant, then copies the verified APK to `dist/`.
- `ink devices` lists human-readable devices, but every subsequent ADB operation targets the device by serial.
- `ink dev [--device <serial-or-unique-name>] [--logs]` reuses the same device selection and starts logs before launching the app.
- `ink logs [--device <serial-or-unique-name>]` shows application output and relevant crash output without clearing the device-wide log buffers.

Gradle, ADB and Logcat are implementation details behind those commands. Keep raw subprocess output behind `--verbose` and make every failure name the failing phase and the useful next action.

## APK signing

### Debug builds

Use the Android Gradle plugin's normal `debug` signing configuration. Android's tools create a developer-local debug keystore at `$HOME/.android/debug.keystore`, and debug certificates are deliberately unsuitable for publishing ([Android app-signing guide](https://developer.android.com/studio/publish/app-signing#debug-mode)). Ink should neither commit nor generate a project-specific debug keystore.

`ink dev` and `ink build --debug` should therefore invoke the debug Gradle variant and accept its automatically signed APK. This preserves install compatibility between both commands on a developer's machine.

### Release builds

Use a separate explicit release/upload key and assign a Gradle `signingConfig` to the release build type. A signing configuration contains the keystore path, keystore password, key alias and key password; Gradle then signs the release artefact as part of the build ([Android app-signing guide](https://developer.android.com/studio/publish/app-signing#configure-gradle)).

Recommended Ink policy:

- `ink build` is the optimised release build.
- If release signing is configured, emit a signed APK.
- If it is not configured, fail with one concise explanation rather than silently signing a supposed release with the debug key.
- Store non-secret metadata such as keystore path and alias in project or user configuration, but source passwords from protected environment variables, standard input or a user-only credentials file. Never put passwords in `ink.toml`, generated Gradle files, command output or command-line arguments. Android explicitly warns that Gradle signing data embedded in build files is plain text and should be moved elsewhere ([key-security guidance](https://developer.android.com/studio/publish/app-signing#secure-shared-keystore)).
- Do not invent or rotate a release key during an ordinary build. Android updates depend on signing continuity, so losing the app-signing key can prevent future updates ([keys and keystores](https://developer.android.com/studio/publish/app-signing#considerations)).

Let Gradle perform normal signing rather than adding a second signing pipeline to the CLI. Use `apksigner` only as a post-build verification step:

```text
apksigner verify --verbose --print-certs dist/<app>-<version>-arm64.apk
```

`apksigner verify` checks platform signature compatibility and can print certificate information ([official `apksigner` reference](https://developer.android.com/tools/apksigner#verify)). If Ink ever signs outside Gradle, use `--ks-pass env:<name>`, `file:<file>` or `stdin`, never `pass:<password>`; all are supported input forms ([key and certificate options](https://developer.android.com/tools/apksigner#options-sign-key-cert)). Perform `zipalign` before signing because modifying an APK afterwards invalidates its signature ([`apksigner` usage warning](https://developer.android.com/tools/apksigner#usage)).

## Device discovery and selection

Start with:

```text
adb devices -l
```

Long output provides the serial, connection state and descriptive `product`, `model` and `device` fields, specifically to distinguish multiple connected devices ([ADB device discovery](https://developer.android.com/tools/adb#devicestatus)). Treat the serial as identity; model names are presentation and may collide.

Selection rules should be deterministic:

1. Keep only entries whose state is `device`.
2. With no `--device`, select automatically only when exactly one usable device exists.
3. With multiple devices, accept an exact serial or a unique case-insensitive match against the displayed model/product label. Otherwise list the candidates and stop.
4. Pass `adb -s <serial>` to every install, launch, property and Logcat command. The ADB manual defines `-s` as the explicit device selector and notes that it overrides `ANDROID_SERIAL` ([AOSP ADB manual](https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md#37)).
5. Remember the last successful serial in user-local Ink state, not in the repository. Reuse it only while that exact serial is connected; never silently fall back to a different device.

`adb devices -l` should normally supply the friendly label. A targeted `adb -s <serial> shell getprop ro.product.manufacturer` and `ro.product.model` can improve presentation when fields are absent. `getprop` is Android's supported shell mechanism for reading system properties ([AOSP system-property guide](https://source.android.com/docs/core/architecture/configuration/add-system-properties#getprop-setprop)). Property lookup failures must not block device use.

## Application logs and crashes

Logcat stores ordinary application, system and crash messages in separate circular buffers; the `default` buffer set includes `main`, `system` and `crash` ([official Logcat guide](https://developer.android.com/tools/logcat#alternativeBuffers)). Ink should not run `logcat -c`, because that destroys unrelated device logs and is unnecessary when a stream begins before launch.

The package-aware path should be:

1. Resolve the installed package's numeric UID.
2. On devices whose Logcat supports it, stream `adb -s <serial> logcat -b default --uid=<uid> -v color,threadtime`. AOSP specifies that `--uid` accepts numeric UIDs and filters messages to them ([AOSP Logcat source/help](https://android.googlesource.com/platform/system/logging/+/c60abd3268cb9c3cc3a872e232f8c991187943dd/logcat/logcat.cpp#357)).
3. Fall back to the current process ID with `--pid=<pid>` where UID filtering is unavailable, and re-resolve/restart the stream after a process restart. PID filtering alone is not a durable package identity.
4. Also inspect the crash buffer for records mentioning the package/process. Native crash reporting can originate from system crash machinery rather than the application UID or PID, so a UID/PID-only stream can omit the useful tombstone header. Present matching crash records adjacent to the app stream and deduplicate repeats.

Ink controls its own native logging, so give framework messages one stable tag such as `Ink`. Tag filters use `tag:priority`, with `*:S` making the specified tags an allowlist ([Logcat filter expressions](https://developer.android.com/tools/logcat#filteringOutput)). A targeted diagnostic fallback can therefore include `Ink:V`, `AndroidRuntime:E`, `DEBUG:E` and `libc:F`, but tag filtering alone is not package-scoped and should not be the normal implementation.

Use `threadtime` for stable timestamps, PID and TID; add `color` only when stderr/stdout is an interactive terminal. These formats and modifiers are part of Logcat's documented interface ([Logcat output format](https://developer.android.com/tools/logcat#format)). `ink dev --logs` should start the stream immediately before app launch so startup failures are captured, and Ctrl-C should terminate the watcher and its ADB children cleanly.

## Implementation order

1. Centralise device discovery/selection and use it from `dev`, `devices` and `logs`.
2. Add `logs` and `dev --logs`, including UID filtering, PID fallback and crash-buffer handling.
3. Separate debug and release signing behaviour, then verify every copied APK with `apksigner`.
This order deepens the existing commands before adding more surface area, while signing and device selection become shared modules rather than duplicated subprocess code.
