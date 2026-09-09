---
title: "Set up your computer and phone"
description: "Install the tools and check your device connection."
---

This is a one-time setup. Ink currently builds Android apps from source on macOS or Linux. The commands below use a POSIX shell, such as zsh or bash.

## Install the tools

Install these tools using their official instructions:

| Tool | Why you need it |
| --- | --- |
| [Git](https://git-scm.com/downloads) | Downloads the Ink checkout |
| [Bun](https://bun.sh/docs/installation) | Installs JavaScript packages and bundles your code |
| [Rust through rustup](https://rustup.rs/) | Compiles the Ink CLI and native engine |
| [JDK 17](https://adoptium.net/temurin/releases/?version=17) | Runs Android's build tools and creates signing keys |
| [Android Studio](https://developer.android.com/studio) | Installs the Android SDK and manages emulators |

Ink pins Rust 1.96.0 in its checkout. Rustup downloads that version when you first run Ink. Set `JAVA_HOME` to your JDK 17 installation if your computer has multiple Java versions.

In Android Studio, open **SDK Manager**. Install Android SDK Platform 36, Android SDK Build-Tools 36.0.0, Platform-Tools, Command-line Tools (latest), and NDK 29.0.14206865. Enable **Show Package Details** to select a specific NDK version.

SDK Manager displays the SDK location. Set `ANDROID_HOME` to that path. Common defaults are:

```sh
# macOS
export ANDROID_HOME="$HOME/Library/Android/sdk"
```

```sh
# Linux
export ANDROID_HOME="$HOME/Android/Sdk"
```

Then add its tools to your shell path:

```sh
export PATH="$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
sdkmanager --licenses
```

Read and accept the Android licences. Add the exports to `~/.zshrc` or `~/.bashrc` to keep them in new terminals. See Android's [sdkmanager guide](https://developer.android.com/tools/sdkmanager) if you prefer installing the SDK packages from the terminal.

## Download Ink

```sh
mkdir -p ~/Developer
git clone https://github.com/vandamd/ink.git ~/Developer/ink
cd ~/Developer/ink
bun install
export PATH="$HOME/Developer/ink/scripts:$PATH"
ink doctor
```

Add the final PATH export to your shell configuration too. This `ink` wrapper selects its own SDK checkout; you do not need a raw Cargo command or a separate SDK environment variable. Use a checkout you have access to if the repository requires authentication.

Resolve any missing tools reported by `ink doctor` before creating an app. A successful check confirms the local tools, not every hardware feature.

## Connect your phone

Enable USB debugging on your Light Phone, connect it with a data-capable USB cable, and accept the debugging authorisation on the phone. Run:

```sh
ink devices
```

An `unauthorized` device needs approval on the phone. If nothing appears, check the cable and USB connection. Linux may also need [Android USB device rules](https://developer.android.com/studio/run/device).

## Or use an emulator

In Android Studio's Device Manager, create an API 34 or newer virtual device named `Light_Phone_III`, with a 1080 × 1240 display. Choose an image supported by your computer: ARM64 on Apple Silicon or x86_64 on an Intel/AMD host. Start it with:

```sh
emulator -avd Light_Phone_III -writable-system
```

A standard Android emulator is enough for basic Ink screens and Android permissions. LightOS-specific services require the Light SDK emulator host; see [LightOS](/light-sdk). A user-reported host workaround is to select **Default**, then **All tools**, in Home's Settings if its tool configuration stops responding. This is separate from whether your app includes LightOS support.

An emulator cannot establish real-phone microphone accuracy, battery use or performance. Use it for UI and development checks, then check the relevant hardware on your phone.

**Next:** [create your app](/standalone#create-an-app).
