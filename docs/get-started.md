---
title: "Get started"
description: "Install Ink, create an app and run it on a Light Phone III."
---

Use macOS Apple Silicon or Linux x64 with glibc 2.35 or newer, such as Ubuntu 22.04. Linux ARM64 supports JavaScript tooling, but cannot build Android apps.

## Install

```sh
curl -fsSL https://ink.noscroll.ing/install.sh | sh
```

The installer includes Bun and runs `ink setup`. Follow its PATH instructions and any prompts for missing build tools. Setup reuses existing tools and asks before installing components.

If setup is incomplete, follow its instructions and run `ink setup` again. `ink doctor` checks prerequisites without installing anything. See [setup options](/releases#install-and-update) for custom tool locations and non-interactive installs.

## Create an app

```sh
ink create my-app --name "My app" --package com.example.myapp
cd my-app
ink check
```

Replace `com.example.myapp` with your app's Android application ID. Ink installs the dependencies and creates the home screen at `app/index.tsx`.

`ink check` checks formatting, lint, types and bundling. Use `ink format` to fix formatting. See [Project structure](/project-structure) for routes, assets and configuration.

## Run on a phone

Enable USB debugging, connect your Light Phone III and accept its debugging authorisation prompt.

```sh
ink devices
ink dev
```

Ink builds, installs and opens the app. Edit `app/index.tsx` to update it on your phone. Press `r` to reload, `a` to reopen the app or **Ctrl+C** to stop. Use `ink logs` to stream app logs.

With several devices connected, use `ink dev --device <serial>`. Ink remembers your selection. `ink dev --once` builds and opens the app without watching for changes.

To use an existing Light Phone III emulator, start it first:

```sh
emulator -avd Light_Phone_III -writable-system
```

This requires an AVD named `Light_Phone_III`. Ink does not create the emulator image.

## Add a module

From your app directory:

```sh
ink add @ink/audio
```

Import the module in your app. Ink installs matching package versions and compiles the native features your imports require. See [Audio](/audio) for usage, or [Light SDK](/light-sdk) for LightOS integration.

Use your own package manager for other JavaScript packages. Ink's bundled Bun is private to the CLI.

## Build an APK

```sh
ink build --debug
```

For a signed APK to distribute, see [Build an APK](/build-app).

## Update

```sh
ink update
```

`ink upgrade` is an alias. The next `ink dev`, `ink build` or `ink check` updates the app's Ink packages. Alpha releases can include breaking changes; the compiler reports any required app changes. Ignoring an update notice keeps your installed version.
