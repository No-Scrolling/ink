---
title: "Build and install a release"
description: "Sign, build and install a release APK."
---

An APK is the file you install on an Android phone. A release APK contains your app without the development reload tools.

## Create a signing key

Android uses a signing key to recognise updates from the same app author. Keep a backup of the key and its password; future updates need them.

Run this once for your app, replacing `my-app` with its name:

```sh
mkdir -p ~/.keystores
keytool -genkeypair -keystore ~/.keystores/my-app-release.jks \
  -storetype JKS -alias release -keyalg RSA -keysize 3072 -validity 10000
```

`keytool`, included in JDK 17, prompts for a password and certificate details. Keep the key outside your app repository.

## Configure signing

Add this to `ink.toml`. Use your actual absolute home path, or a path relative to `ink.toml`:

```toml
[signing]
keystore = "/Users/your-name/.keystores/my-app-release.jks"
key_alias = "release"
```

Set `INK_KEYSTORE_PASSWORD` in your shell environment, then run from your app folder:

```sh
ink check
ink build
```

For example, in bash you can prompt without displaying the password or writing it in shell history:

```bash
read -r -s -p 'Keystore password: ' INK_KEYSTORE_PASSWORD
export INK_KEYSTORE_PASSWORD
ink build
unset INK_KEYSTORE_PASSWORD
```

In zsh, use `read -r -s 'INK_KEYSTORE_PASSWORD?Keystore password: '` instead. Set `INK_KEY_PASSWORD` too if the key password differs from the keystore password.

The build prints the APK path under `dist/`, normally `<name>-<version>-arm64.apk`. Development builds still use their development key even when release signing is configured.

## Install it

```sh
ink devices
adb -s <serial> install -r dist/<your-release-file>.apk
```

Replace both placeholders with your device serial and the exact file printed by the build. The `-r` option updates an installed app while retaining its data when its package identifier and signing key match.

A release key cannot replace a development-signed app with the same identifier. Use a distinct identifier for a separate installation, or export any data you need before uninstalling the development app. Uninstalling removes its app data.

For an update, keep the package identifier and signing key, increase `version_code` in `ink.toml`, and update the human-readable `version`.

Before sharing, try a fresh install, an update, permission denial, a failed network request and reopening after the app was stopped. Compilation alone cannot verify those interactions.
