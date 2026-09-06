# Using Ink outside this repository

The current SDK distribution is an explicit local checkout. It is not a published npm release. Keep the checkout intact: it contains the compiler, native sources, Android project, fonts and packages. `sdk.json` records its framework/protocol and React version set.

Use the checkout's `scripts/ink` from any directory. It selects that SDK automatically. For a separately compiled CLI, set `INK_SDK_ROOT` to the checkout, or place the SDK at `$XDG_CONFIG_HOME/ink/sdk/current` (default `~/.config/ink/sdk/current`). There is no compiled-in developer path.

```sh
/path/to/ink/scripts/ink create ~/Developer/my-app --name "My App" --package com.example.myapp
cd ~/Developer/my-app
bun install
/path/to/ink/scripts/ink doctor
/path/to/ink/scripts/ink check
/path/to/ink/scripts/ink dev --device emulator-5554
```

Creation refuses to overwrite an existing directory. Dependencies use explicit local package paths and a compatible React version; it does not pretend unpublished package names are available from a registry. Additional `@ink/*` packages can be installed from the corresponding SDK package directories. Use the shared `ink/tsconfig` so app types describe implemented runtime facilities rather than the entire browser DOM.

Per-app generated assets, Gradle project cache and Android outputs live in `.ink`. The SDK's Cargo cache is shared; CLI native builds take a filesystem lock while producing/copying native artefacts. Apps do not overwrite each other's APKs. `.ink`, `dist`, dependencies and signing keystores are ignored by the generated `.gitignore`.

For release builds, configure your own keystore in `ink.toml` using the compiler's signing configuration and supply `INK_KEYSTORE_PASSWORD` (optionally `INK_KEY_PASSWORD`) through the environment. Run `ink build`; release signing never uses the development key implicitly. Publishing packages, choosing public registry names and distributing signed releases remain explicit release decisions.

The first distribution requires Bun, Rust, JDK 17, the Android SDK and NDK. `ink doctor` diagnoses those requirements. Moving an app to another machine requires updating its local SDK dependency paths; a registry or packaged SDK distribution is future release work, not a runtime compatibility guarantee.
