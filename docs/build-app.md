---
title: "Build an APK"
description: "Build and sign an Android app for distribution."
---

Run build commands from your app directory after completing [setup](/get-started).

## Development build

```sh
ink build --debug
```

This builds a development APK without a release signing key.

## Release build

Create a signing key once and keep a backup. Updates to an installed app must use the same key.

```sh
keytool -genkeypair -keystore release.jks -alias app -keyalg RSA -keysize 3072 -validity 10000
```

Add its location to `ink.toml`:

```toml
[signing]
keystore = "release.jks"
key_alias = "app"
```

Set `INK_KEYSTORE_PASSWORD` in your shell or secret manager. If the key has a different password, set `INK_KEY_PASSWORD` too. Otherwise, Ink uses the keystore password.

```sh
ink build
```

Ink writes the signed APK to `dist/`. Keep keystores and passwords out of version control.

Before distributing an update, increase `version_code` in `ink.toml`. Use `version` for the human-readable version number.

`ink info` shows configuration and built APKs without compiling. Its cached JavaScript size and capabilities describe the last successful compilation; run `ink check` to refresh them.
