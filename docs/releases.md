---
title: "Releases"
description: "Build, verify and publish an Ink release."
---

An Ink release contains the CLI, a private Bun executable, JavaScript build tools, native source and Android templates. Each app compiles its selected native features. Java, Rust and the Android SDK/NDK remain external prerequisites.

Release archives cover macOS Apple Silicon, Linux x64 and Linux ARM64. Linux requires glibc 2.35 or newer, such as Ubuntu 22.04 or later; Alpine/musl is not supported. Windows and Intel Macs are not release targets. Android apps target Light Phone III arm64 devices.

Linux ARM64 supports the CLI and JavaScript tooling, including package installation and `ink check`. Android builds require Linux x64 or macOS Apple Silicon: [Google's Linux SDK/NDK host tools are x64](https://developer.android.com/ndk/guides/other_build_systems). `ink setup`, `ink doctor` and Android build commands report this limitation on Linux ARM64.

## Build a release locally

Install the repository prerequisites and the Bun version declared in `sdk.json`:

```sh
bun install --frozen-lockfile
cargo build --locked --release -p ink-cli
bun scripts/release/pack.ts
```

The output directory is `target/distribution`. It contains the platform archive, `SHA256SUMS` and npm archives under `npm/`. The packer includes selected source files, including uncommitted edits, so publish only from a reviewed commit through the release workflow.

Build on the target host: `ink-darwin-arm64.tar.gz`, `ink-linux-x64.tar.gz` or `ink-linux-arm64.tar.gz`. Each archive contains Bun and native JavaScript tooling for that host.

Install that archive into an isolated directory. On Linux, replace the filename below with the archive for your architecture:

```sh
INK_HOME="$PWD/target/local-install" \
INK_BIN_DIR="$PWD/target/local-bin" \
INK_ARCHIVE="$PWD/target/distribution/ink-darwin-arm64.tar.gz" \
sh scripts/install.sh

target/local-bin/ink --version
target/local-bin/ink doctor
```

The CLI resolves its SDK relative to its executable. It uses the bundled Bun for its commands without replacing a global Bun installation. Package names are `ink-framework` and `ink-framework-<module>`; generated apps use npm aliases to preserve `ink` and `@ink/*` imports. Before the first public release, use a local npm registry containing the generated archives to verify app installation.

## Set up publishing

The repository is currently private. Make `No-Scrolling/ink` public at launch before advertising the installer or update command; anonymous GitHub downloads cannot read private releases. Preparing and verifying archives does not require changing visibility.

The GitHub repository must allow Actions to create pull requests. Create a GitHub environment named `release` and configure its publishing permissions as required.

An npm maintainer must establish ownership of `ink-framework` and each `ink-framework-*` package. Check availability before the first publication, authenticate locally with `npm login`, then publish the generated package archives.

After the packages exist, configure npm trusted publishing on every package:

- GitHub owner: `No-Scrolling`
- Repository: `ink`
- Workflow filename: `release.yml`
- Environment: `release`
- Allow direct `npm publish`

The release workflow uses GitHub OIDC, so subsequent releases do not require a stored npm token. See [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/). Complete the first trusted publication within npm's configuration expiry window.

## Publish a version

In GitHub Actions, run **Prepare Release** with a version such as `0.1.0-alpha.2`. It updates the SDK, package and CLI versions and lockfiles, then opens a release pull request against `main`.

Review and merge the pull request. **Release** first creates the version tag at the release commit, then builds all three platform archives on native GitHub runners. It checks installation and bundled JavaScript tooling, verifies that the tag still matches the build commit, publishes the npm packages once, then creates the GitHub prerelease with all downloads and a combined checksum file. All platforms must pass before publishing. All packages use the same version. npm's `latest` tag is intentional during the alpha; it does not promise API stability.

The workflow waits up to ten minutes for npm's full and install-specific package listings to expose every published version with the expected checksum before creating the GitHub release. If npm takes longer, the job fails without publishing the CLI release; retry the failed job later.

To verify packaging without publishing, run **Release** manually and leave **Publish packages and the GitHub release after verification** unchecked. Download the resulting platform and npm artifacts from the workflow run. Enable that option only when manually publishing or retrying a release.

If a build or publishing step fails, rerun the failed jobs on the same commit. The version tag may already exist; the workflow reuses it only if it points to that commit. Already published package versions are skipped. Do not reuse a published version for changed contents.

## Install and update

After the first GitHub release is published:

```sh
curl -fsSL https://ink.noscroll.ing/install.sh | sh
ink create my-app
cd my-app
ink dev
```

The installer adds only Ink's installation and command symlink. It prints a PATH instruction when necessary and leaves shell configuration untouched. `INK_HOME` and `INK_BIN_DIR` select alternative locations. Existing unrelated commands are not overwritten.

Mintlify redirects `/install.sh` to `scripts/install.sh` on GitHub through `docs/docs.json`. The endpoint must be accessible without signing in. After deploying a redirect change, download it with `curl -fsSL https://ink.noscroll.ing/install.sh -o /tmp/ink-install.sh` and check that the response is the shell script before advertising the URL.

Interactive installs run `ink setup` automatically, which asks before installing missing tools. CI and non-interactive installs skip setup; set `INK_SKIP_SETUP=1` to skip it explicitly. Run `ink setup` later if prerequisites remain incomplete. Linux ARM64 skips Android setup because its host tools are unsupported.

`ink setup` reuses existing tools and asks before installing missing Rust versions, the Android Rust target, cargo-ndk or Android SDK packages. Java, rustup and Android command-line tools have manual installation guidance when absent. Ink leaves global defaults unchanged. `ink doctor` performs the same checks without installing anything.

```sh
ink update
```

`ink upgrade` is an alias for `ink update`.

This installs the newest published Ink release, including alpha releases. The next `ink dev`, `ink build`, `ink check`, `ink format`, `ink lint`, `ink export`, `ink add` or `ink install` synchronises the project's Ink dependencies with the installed release. Other dependency version declarations remain unchanged. API incompatibilities are reported by the normal compiler and checking tools; alpha upgrades are not blocked on app compatibility.

Native commands check prerequisites before changing package declarations. A failed dependency installation restores `package.json` and the text lockfile; rerun `ink install` to repair a partially updated `node_modules`. An interrupted process may leave `.ink/install.lock`; remove it only after confirming no install is running.

Update notices use a daily cached check in interactive terminals and are disabled in CI. The first check runs in the background, so a notice can appear on the next command. Ignoring a notice leaves the installed release unchanged. CI should install an explicit release with `INK_VERSION` and use its commands.

## Develop from a checkout

Install Git, Bun, Rust through rustup, Java 17 and the Android SDK command-line tools. Set `JAVA_HOME` and `ANDROID_HOME` for those installations and put Android's `platform-tools` on `PATH`.

```sh
git clone https://github.com/No-Scrolling/ink.git
cd ink
bun install
rustup toolchain install 1.96.0 --component clippy --component rustfmt --target aarch64-linux-android
cargo +1.96.0 install cargo-ndk --version 4.1.2 --locked
sdkmanager "platform-tools" "platforms;android-36" "build-tools;36.0.0" "ndk;29.0.14206865"
sdkmanager --licenses
./scripts/ink doctor
./scripts/ink -C examples/light-template dev
```

Source-checkout development uses `scripts/ink` and local `file:` dependencies. App commands resolve their SDK from the app's `ink` dependency. When the launcher belongs to another checkout, build and editing commands forward to the selected SDK's launcher. `INK_SDK_ROOT` selects the SDK for app creation and environment checks only. Installed releases use registry packages and never dispatch through another source checkout.

To move an app between checkouts, update its `ink` and `@ink/*` `file:` dependencies in `package.json`, remove obsolete Ink `overrides` produced by older scaffolds, then run the new checkout's `scripts/ink add` with your existing modules and `scripts/ink check` from the app directory. Apps without modules can run `bun install` instead. All Ink packages must come from that checkout. Workspace examples use their enclosing SDK.
