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

Review and merge the pull request. **Release** builds all three platform archives on native GitHub runners, checks installation and bundled JavaScript tooling, publishes the npm packages once, then creates the GitHub tag and prerelease with all downloads and a combined checksum file. All platforms must pass before publishing. All packages use the same version. npm's `latest` tag is intentional during the alpha; it does not promise API stability.

To verify packaging without publishing, run **Release** manually and leave **Publish packages and the GitHub release after verification** unchecked. Download the resulting platform and npm artifacts from the workflow run. Enable that option only when manually publishing or retrying a release.

If publishing fails partway through, rerun the workflow on the same commit. Already published package versions are skipped. Do not reuse a published version for changed contents.

## Install and update

After the first GitHub release is published:

```sh
curl -fsSL https://raw.githubusercontent.com/No-Scrolling/ink/main/scripts/install.sh | sh
ink create my-app
cd my-app
ink dev
```

The installer adds only Ink's installation and command symlink. It prints a PATH instruction when necessary and leaves shell configuration untouched. `INK_HOME` and `INK_BIN_DIR` select alternative locations. Existing unrelated commands are not overwritten.

Interactive installs run `ink setup` automatically, which asks before installing missing tools. CI and non-interactive installs skip setup; set `INK_SKIP_SETUP=1` to skip it explicitly. Run `ink setup` later if prerequisites remain incomplete. Linux ARM64 skips Android setup because its host tools are unsupported.

`ink setup` reuses existing tools and asks before installing missing Rust versions, the Android Rust target, cargo-ndk or Android SDK packages. Java, rustup and Android command-line tools have manual installation guidance when absent. Ink leaves global defaults unchanged. `ink doctor` performs the same checks without installing anything.

```sh
ink update
```

`ink upgrade` is an alias for `ink update`.

This installs the newest published Ink release, including alpha releases. The next `ink dev`, `ink build`, `ink check`, `ink format`, `ink lint`, `ink export`, `ink add` or `ink install` synchronises the project's Ink dependencies with the installed release. Other dependency version declarations remain unchanged. API incompatibilities are reported by the normal compiler and checking tools; alpha upgrades are not blocked on app compatibility.

Native commands check prerequisites before changing package declarations. A failed dependency installation restores `package.json` and the text lockfile; rerun `ink install` to repair a partially updated `node_modules`. An interrupted process may leave `.ink/install.lock`; remove it only after confirming no install is running.

Update notices use a daily cached check in interactive terminals and are disabled in CI. The first check runs in the background, so a notice can appear on the next command. Ignoring a notice leaves the installed release unchanged. CI should install an explicit release with `INK_VERSION` and use its commands.

Source-checkout development continues to use `scripts/ink` and local `file:` dependencies. Installed releases use registry packages and never dispatch through another source checkout.
