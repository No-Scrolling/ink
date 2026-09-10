---
title: "Installation"
description: "Install Ink and create an app."
---

Ink is a framework for building Light Phone III apps with React and TypeScript. It currently runs from a local SDK checkout; there is no published installer or npm release yet.

Requires macOS or Linux.

## Install the tools

| Tool | Why you need it |
| --- | --- |
| [Git](https://git-scm.com/downloads) | Downloads the Ink checkout |
| [Bun](https://bun.sh/docs/installation) | Installs JavaScript packages and bundles your code |
| [Rust through rustup](https://rustup.rs/) | Compiles the Ink CLI and native engine |
| [JDK 17](https://adoptium.net/temurin/releases/?version=17) | Runs Android's build tools and creates signing keys |
| [Android Studio](https://developer.android.com/studio) | Installs the Android SDK and manages emulators |

Rustup installs Ink’s pinned Rust version on first use. Set `JAVA_HOME` to your JDK 17 installation if your computer has multiple Java versions.

In Android Studio, open **SDK Manager**. Install Android SDK Platform 36, Android SDK Build-Tools 36.0.0, Platform-Tools, Command-line Tools (latest), and NDK 29.0.14206865. Enable **Show Package Details** to select a specific NDK version.

Set `ANDROID_HOME` to the location shown in SDK Manager:

```sh
# macOS
export ANDROID_HOME="$HOME/Library/Android/sdk"
```

```sh
# Linux
export ANDROID_HOME="$HOME/Android/Sdk"
```

```sh
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
sdkmanager --licenses
```

Accept the licences when prompted. Add the exports to `~/.zshrc` or `~/.bashrc` to keep them in new terminals.

## Download Ink

Repository access is required.

```sh
mkdir -p ~/Developer
git clone https://github.com/vandamd/ink.git ~/Developer/ink
cd ~/Developer/ink
bun install
export PATH="$HOME/Developer/ink/scripts:$PATH"
ink doctor
```

Add the final PATH export to your shell configuration. Install any missing tools reported by `ink doctor`.

## Connect your phone

Enable USB debugging, connect your Light Phone by USB and accept its authorisation prompt.

```sh
ink devices
```

An `unauthorized` device needs approval on the phone. If nothing appears, check the cable and USB connection. Linux may also need [Android USB device rules](https://developer.android.com/studio/run/device).

## Or use an emulator

In Android Studio's Device Manager, create an API 34 or newer virtual device named `Light_Phone_III`, with a 1080 × 1240 display. Choose an image supported by your computer: ARM64 on Apple Silicon or x86_64 on an Intel/AMD host.

```sh
emulator -avd Light_Phone_III -writable-system
```

A standard Android emulator supports basic Ink screens and Android permissions. LightOS services require the [Light SDK emulator host](/light-sdk).

Check microphone accuracy, battery use and performance on a physical phone.


## Create an app

```sh
ink create ~/Developer/my-app --name "My App" --package com.example.myapp
cd ~/Developer/my-app
bun install
ink check
ink devices
```

The folder must not already exist. `com.example.myapp` is the app's unique Android identifier; choose your own before distributing it.

Use the device serial listed by `ink devices`:

```sh
ink dev --device emulator-5554
```

Replace `emulator-5554` with your device’s serial. Keep `ink dev` running while editing.

The app opens to **Home**, displaying **Welcome to Ink!**.

**Next:** [project structure](/project-structure).
