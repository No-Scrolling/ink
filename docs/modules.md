---
title: "Packages"
description: "Install normal npm libraries, reusable Ink UI and native integrations."
tag: "Design specification"
---

An Ink app is a TypeScript project with a package.json and lockfile. Use your preferred package manager. A package can provide ordinary functions, components, a provider client or a native integration; it does not need a special Ink source language.

## Add JavaScript

```sh
bun add your-preferred-parser
```

Import its normal exports. Packages using only the [supported host environment](runtime.md) run in the app's JavaScript runtime. Type declarations improve authoring but do not replace runtime validation.

Pure JavaScript packages need no native manifest, custom export condition, compiler IR or Ink-specific publishing command.

## Add a native package

```sh
ink add @ink/barcode
```

`ink add` installs the package with the project's package manager and registers its native entry points in `ink.toml`. Review that config alongside the lockfile. A rebuild includes the native implementation; a JavaScript reload cannot add missing native code.

```toml
[android]
modules = ["@ink/barcode/generate", "@ink/barcode/scan"]
```

Packages expose separate native entry points where their implementation can actually be split. A passes app that only displays codes can register generation alone. A scanner includes camera support. Removing a JavaScript import does not remove an explicitly registered native package; remove its config entry too.

`ink info` explains the resulting graph: JavaScript bundle, native libraries, permissions, Android components, assets and shared dependencies. It reports actual grouping, not a promise that every native method is independently removable.

## Choose the right layer

| Need | Package |
| --- | --- |
| Weather API, RSS parsing, domain models | Ordinary TypeScript with fetch and a decoder. |
| Reusable settings or playlist screens | TSX compiled for the standard React JSX runtime; `react` and `ink` as peer dependencies. |
| A BLE device protocol | TypeScript over `@ink/bluetooth`. |
| A music provider or messaging SDK | A provider package, with native code where its protocol requires it. |
| Camera, audio codec, map renderer | A native Ink package. |

Keep provider-specific concepts in provider packages. `@ink/audio` plays supported media; it does not promise to implement Spotify authentication, DRM, Connect, private protocols or a provider's offline rights.

## Package conventions

Prefer async functions for one-off work and immutable snapshots plus commands for ongoing state. Accept an `AbortSignal` when cancellation applies. A reusable domain function should work outside a component; offer an optional hook for UI convenience.

Do not force consumers to adopt your state library unless that is explicitly the package's purpose.

Packages execute with the app's available capabilities. Namespaced settings and record IDs prevent accidental collisions, not malicious access. Native packages are ordinary native code in the APK.

## Local development

Use workspace dependencies for modules you are building alongside an app. A local `weather.ts` can become a package without rewriting its functions into another model. Compile TSX with the standard React JSX runtime and publish normal JavaScript plus declarations.

Provider preview fixtures are useful for offline, empty and expired-account states. They are ordinary injected dependencies; using a fixture does not prove the real provider or device integration works.

See [Publish a package](developing-modules.md) and [the weather walkthrough](open-meteo-module.md).
