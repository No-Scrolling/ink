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

Release APKs use `<app-name>-<version>.apk`; development APKs add `-debug`.

Before distributing an update, increase `version_code` in `ink.toml`. Use `version` for the human-readable version number.

`ink info` shows configuration and built APKs without compiling. Its cached JavaScript size and capabilities describe the last successful compilation; run `ink check` to refresh them.

## Release through GitHub Actions

New apps created with `ink create` include **Prepare Release** and **Release** workflows. Add them to an existing app with:

```sh
ink setup release
```

The workflows use `main`. Setup refuses to overwrite files with different contents; move those files aside before regenerating, then merge any customisations.

Run `ink install` and commit `package.json` and `bun.lock` to pin the app to a published Ink version.

Create a signing key as described above, then add its contents and password as repository Actions secrets:

```sh
base64 < ~/.keystores/app-release.jks | gh secret set INK_KEYSTORE_BASE64
gh secret set INK_KEYSTORE_PASSWORD
```

Run these commands from the app's GitHub checkout. The password command prompts privately. If the key has a separate password, add `INK_KEY_PASSWORD` too. Set the repository variable `INK_KEY_ALIAS` if the alias differs from `app`.

Allow GitHub Actions to create pull requests in the repository's Actions settings. Run **Prepare Release** with a version newer than the current app version. It opens a PR updating `version` and incrementing `version_code` in `ink.toml`.

Merge the PR to release. The workflow reserves the version tag, installs the pinned Ink release and its build tools, runs the app's `check` script, then builds and verifies the signed APK. The GitHub release contains the APK and `SHA256SUMS`. Versions with a prerelease suffix produce GitHub prereleases.

To try the build first, run **Release** manually with publishing unchecked. Download the `signed-apk` workflow artifact when it completes. This does not create a tag or GitHub release.

Rerun failed jobs on the same commit; a tag pointing elsewhere requires a new version. Signing configuration stays on the runner, and the temporary keystore is removed when the job finishes.
