---
title: "Get started"
description: "Install Ink, create an app and run it on a Light Phone III."
---

Build Android apps on macOS Apple Silicon or Linux x64. Linux release archives require glibc 2.35 or newer, such as Ubuntu 22.04. The Linux ARM64 CLI supports JavaScript tooling, but Android builds are not supported because Google's Linux SDK/NDK host tools require x64.

## Install Ink

```sh
curl -fsSL https://ink.noscroll.ing/install.sh | sh
```

The installer downloads the CLI and a private copy of Bun, then runs `ink setup` in interactive terminals. You do not need to clone Ink or install Bun separately. Follow any printed PATH instructions and keep that change in your shell configuration.

Setup reuses existing Rust, Java and Android tools and asks before installing missing components. If rustup, Java or Android SDK command-line tools are missing, follow its installation instructions, then run `ink setup` again. Set `JAVA_HOME` and `ANDROID_HOME` if those tools are installed in custom locations. Ink leaves global toolchain defaults unchanged.

CI and non-interactive installs skip setup. Run `ink setup` manually when needed; `ink doctor` checks prerequisites without installing anything. Resolve missing prerequisites before building an Android app.

## Create an app

Choose a unique Android application ID:

```sh
ink create my-app --name "My app" --package com.example.myapp
cd my-app
ink check
```

`ink create` installs the app's dependencies. `app/index.tsx` is the home screen. See [Project structure](/project-structure) for layouts, assets and environment variables. `ink check` checks formatting, lint, TypeScript and bundling without changing source files. Use `ink format` to apply formatting.

## Run on a phone

Enable USB debugging on the phone, connect it and accept its debugging authorisation prompt.

```sh
ink devices
ink dev
```

When several devices are connected, select one with `ink dev --device <serial>`. Ink remembers the selection. Edit the home screen to update the running app. Press `a` to open it, `r` to reload, or Ctrl-C to stop watching. `ink dev --once` builds and launches once; `ink logs` streams app logs.

For the project's Light Phone III emulator configuration, launch `emulator -avd Light_Phone_III -writable-system` first. This requires an existing AVD named `Light_Phone_III`; Ink does not create the emulator image.

## Add a module

Add an Ink module from your app directory:

```sh
ink add @ink/audio
```

Ink installs the requested module and its Ink dependencies at the installed release version. Import the module you use; Ink compiles the native features required by your app. Use `import "@ink/network"` in modules that use `fetch` or other network globals. Importing `@ink/lightos` enables LightOS integration. Use your own package manager for other JavaScript packages; Ink's bundled Bun is private to the CLI.

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

## Update Ink

```sh
ink update
```

`ink upgrade` does the same thing. The next `ink dev`, `ink build` or `ink check` synchronises the app's Ink packages with the installed release. Alpha releases can introduce breaking changes; the compiler reports any app changes needed. Ignoring an update notice leaves the installed version unchanged.

`ink info` shows configuration and built APKs without compiling. JavaScript size and capabilities are labelled cached and describe the last successful compilation; run `ink check` to refresh them.

For framework development from a checkout and release publishing, see [Releases](/releases).
