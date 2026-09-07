---
title: "Create an app"
description: "Create, configure and build your first Ink app."
---

Ink currently uses a local SDK checkout, not a published npm release. Keep the checkout: it contains the compiler, Android project, native code, fonts and packages.

## Prerequisites

Install Bun, Rust, JDK 17, the Android SDK and the NDK. Run `scripts/ink doctor` from the SDK checkout to check your setup.

## Create and run

Replace `/path/to/ink` with your SDK checkout:

```sh
/path/to/ink/scripts/ink create ~/Developer/my-app --name "My App" --package com.example.myapp
cd ~/Developer/my-app
bun install
/path/to/ink/scripts/ink check
/path/to/ink/scripts/ink dev --device emulator-5554
```

Use `ink devices` to find another device serial. Creation requires a new directory and will not overwrite an existing one.

Generated apps use local package paths and a compatible React version. Add other `@ink/*` packages from the SDK’s package directories. Extend `ink/tsconfig` so TypeScript uses Ink’s runtime types.

## Add features

Use `ink` for components and navigation. Add optional packages for the features your app needs:

| Package | Features |
| --- | --- |
| `@ink/audio` | Playback; recording through `/capture` |
| `@ink/auth` | Sign-in and account tokens |
| `@ink/background` | Background jobs |
| `@ink/barcode` | Generated codes; scanning through `/scan` |
| `@ink/camera` | Camera preview and photos |
| `@ink/clipboard` | Clipboard access |
| `@ink/files` | File operations; photo/video picker through `/media` |
| `@ink/lightos` | LightOS services |
| `@ink/location` | Location fixes and tracking |
| `@ink/maps` | Maps |
| `@ink/network` | Web globals; `/connectivity` and `/downloads` for native network features |
| `@ink/nfc` | NFC tags |
| `@ink/notifications` | Notifications |
| `@ink/secure-store` | Encrypted secrets |
| `@ink/store` | Persisted app data and read-only databases |

Related features share a package, but their entry points select native capabilities independently. For example, generating a barcode does not include the camera or scanner.

## Choose the SDK

`scripts/ink` selects its own checkout. If you use a separately compiled CLI, set `INK_SDK_ROOT` or install the SDK at `$XDG_CONFIG_HOME/ink/sdk/current` (default `~/.config/ink/sdk/current`).

When moving an app to another machine, update its local package paths and SDK location. `sdk.json` records the SDK’s framework, protocol and React versions.

## Build a release

Configure a keystore in `ink.toml`, set `INK_KEYSTORE_PASSWORD` and run `ink build`. If the key has a separate password, set `INK_KEY_PASSWORD` too. See [release signing](/ink#build-and-inspect).

Each app keeps its generated files, Gradle cache and Android output in `.ink`. The SDK shares its Cargo cache and locks native builds to prevent conflicting writes. Generated `.gitignore` rules exclude build output, dependencies and signing keys.
