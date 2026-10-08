---
title: "Get started"
description: "Install the local SDK, create an app and run it on Android."
---

Ink currently builds from a local SDK checkout. Install Git, Bun, Rust through rustup, Java 17 and the Android SDK command-line tools. Set `JAVA_HOME` and `ANDROID_HOME` for those installations and put Android's `platform-tools` on `PATH`.

## Install the SDK

```sh
git clone https://github.com/vandamd/ink.git
cd ink
bun install
rustup toolchain install 1.96.0 --component clippy --component rustfmt --target aarch64-linux-android
sdkmanager "platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;29.0.14206865"
sdkmanager --licenses
mkdir -p "$HOME/.local/bin"
ln -s "$PWD/scripts/ink" "$HOME/.local/bin/ink"
export PATH="$HOME/.local/bin:$PATH"
ink doctor
```

Keep the `PATH` change in your shell configuration. The launcher uses its checkout to create apps. `ink doctor` reports missing tools and connected devices; resolve its reported problems before continuing.

## Create an app

Choose a unique Android application ID. From the SDK checkout:

```sh
ink create ../my-app --name "My app" --package com.example.myapp
cd ../my-app
bun install
ink check
```

`app/index.tsx` is the home screen. See [Project structure](/project-structure) for layouts, assets and environment variables. `ink check` checks formatting, lint, TypeScript and bundling without changing source files. Use `ink format` to apply formatting.

## Run on a phone

Enable USB debugging on the phone, connect it and accept its debugging authorisation prompt.

```sh
ink devices
ink dev
```

When several devices are connected, select one with `ink dev --device <serial>`. Ink remembers the selection. Edit the home screen to update the running app. Press `a` to open it, `r` to reload, or Ctrl-C to stop watching. `ink dev --once` builds and launches once; `ink logs` streams app logs.

For the project's Light Phone III emulator configuration, launch `emulator -avd Light_Phone_III -writable-system` first. This requires an existing AVD named `Light_Phone_III`; Ink does not create the emulator image.

## Add a module

Modules use the same SDK checkout as your `ink` dependency:

```sh
ink add @ink/audio
```

Ink installs the requested module and resolves its local dependencies. Import the module you use; Ink includes its native requirements automatically. Use `import "@ink/network"` in modules that use `fetch` or other network globals. Importing `@ink/lightos` enables LightOS integration. Use `bun add` for ordinary JavaScript packages.

## Build an APK

For a development APK without release signing:

```sh
ink build --debug
```

For release builds, create a signing key once and keep a backup. Future updates to the installed app require the same key.

```sh
keytool -genkeypair -keystore release.jks -alias app -keyalg RSA -keysize 3072 -validity 10000
```

Add the key's location to `ink.toml`:

```toml
[signing]
keystore = "release.jks"
key_alias = "app"
```

Set `INK_KEYSTORE_PASSWORD` in your shell or secret manager. Set `INK_KEY_PASSWORD` too if the key has a different password; otherwise Ink uses the keystore password. Then build:

```sh
ink build
```

The signed APK is written to `dist/`. Keep keystores and passwords out of version control. Increase `version_code` in `ink.toml` before distributing an update; `version` is its human-readable version.

## Change SDK checkout

App commands resolve their SDK from the app's `ink` dependency, independently of `INK_SDK_ROOT`. When the launcher belongs to another checkout, build and editing commands forward to the selected SDK's launcher. That environment variable selects the SDK for creation and environment checks only.

To move an existing app to another checkout, update its `ink` and `@ink/*` `file:` dependencies in `package.json`, remove obsolete Ink `overrides` produced by older scaffolds, then run `ink add` with your existing `@ink/*` modules and `ink check`. Apps without modules can run `bun install` instead. All Ink packages must come from that checkout. Workspace examples use their enclosing SDK.

`ink info` shows configuration and built APKs without compiling. JavaScript size and capabilities are labelled cached and describe the last successful compilation; run `ink check` to refresh them.
