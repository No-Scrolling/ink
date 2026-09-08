---
title: "Builds, native requirements and assets"
description: "How Ink resolves native requirements, assets and release builds."
---

## Product contract

Include native functionality by explicit package entry point, never by inspecting hook arguments or component names. Import `@ink/audio` for playback and `@ink/audio/microphone` for analysis or `@ink/audio/recording` for recording. Camera capture uses `@ink/camera`; scanning uses `@ink/barcode/scan`. A playback-only app does not require microphone permission or include recorder code, and a capture-only app does not include the barcode scanner.

`@ink/files` provides managed file operations. `@ink/files/images` adds image preparation; `@ink/files/media` adds the photo/video gallery and thumbnail rendering. Basic files and recordings do not require either image feature.

`openURL` and `share` from `ink`, and `@ink/auth`, include the `external` capability for browser and sharing windows. Browser package queries are included only with this capability. The playback service is declared only for `audio-detached`.

Networking follows the same rule: `@ink/network` installs web globals, while `/connectivity` and `/downloads` include only their respective native features. UI imports from `ink` do not install web globals. Development compiler tools live in the SDK workspace’s development dependencies, separate from app runtime dependencies.

## React compilation

Ink uses Oxc's React Compiler for app source in development and release builds. It automatically memoises eligible components and hooks; no app configuration is needed. Dependencies and generated entry points are excluded.

Oxc runs React compilation, TypeScript removal, Fast Refresh instrumentation, JSX transformation and export analysis. Bun bundles the result and minifies release builds. During development, Bun also converts persistent modules so their state survives component updates. These builds reuse transformations across passes and compose source maps back to the original source. Only imported files are processed. The compiler runs on your computer, not on the phone; its generated caching code runs with React in the app.

Write components and hooks using the Rules of React. Compilation does not replace TypeScript checking or guarantee that every function is optimised. Ink pins the Oxc version because its React Compiler integration is experimental.

## Declare native requirements

The bundler follows resolved imports, including re-exports, aliases, linked packages and worker code. Renaming an imported component does not change its native requirements. Release builds use the bundler’s output metadata to select retained modules; they do not inspect JSX props. Use `ink info` to inspect the result.

Each supported package ships `ink-native.json` beside `package.json`:

```json
{
  "version": 1,
  "modules": {
    "src/index.ts": ["audio-detached"],
    "src/microphone.ts": ["audio-microphone"],
    "src/recording.ts": ["audio-recording"],
    "src/microphone-permission.ts": ["microphone-permission"]
  }
}
```

Paths are relative to the resolved package directory. `*` applies to each contributing module. Named entries apply when that implementation file contributes code to the release bundle. Modules removed by tree shaking do not add capabilities. Development builds conservatively include loaded modules so refresh can use their exports. This supports package export splits without depending on how a consumer names an import. A resolved Ink package missing its declaration fails with an installation/migration diagnostic; unknown capabilities fail during metadata decoding. Ordinary JavaScript dependencies do not require declarations. Third-party packages may provide declarations too.

`crates/ink-compiler/capabilities-v1.json` owns capability costs, dependency closure, permissions and Android source groups. The compiler resolves dependencies after adding explicit `ink.toml` capabilities. Gradle consumes the same catalogue and retains build actions and SDK dependency wiring.

## Import assets

Import local images and MP3 audio. Use HTTPS or managed-file URLs for runtime sources.

```tsx
import photo from './photo.jpg';
import track from './track.mp3';
<Image src={photo} width={200} height={200} />
```

## Import icons

Import Material Symbols directly from `ink/icons`:

```tsx
import { settings, favorite, favoriteFilled } from "ink/icons";
import { Icon, Button } from "ink";

<Button icon={settings}>Settings</Button>
<Icon name={active ? favoriteFilled : favorite} size={28} />
```

Names use camel case: `more_horiz` becomes `moreHoriz`. The plain export is outlined; the `Filled` suffix selects the filled variant. TypeScript provides completion and catches unknown exports without a separate collection file or generation step.

Use named imports. Release builds remove unused exports and generate assets only for icon references remaining in the bundle. Importing only `favoriteFilled` includes only that variant. Switching between `favorite` and `favoriteFilled` retains both. References work through props, arrays and objects as ordinary TypeScript values.

For dynamic lookups, build a small object from the icons your app supports and use `findIcon(collection, externalName)` from `ink`. It returns `undefined` for an unknown key. Avoid a namespace import used dynamically: it can retain the entire catalogue.

Ink owns raster resolution and generates icons at 56 logical units, covering its standard controls. The `Icon` component's `size` controls display size; larger sizes can soften edges. Icons use weight 400; the native keyboard uses its separate weight-300 assets. Back and input-clear icons are always included.

Names starting with a digit have an `icon` prefix, such as `icon360`. JavaScript reserved names and catalogue names already ending in `Filled` have an `Icon` suffix, such as `deleteIcon`, to keep exports valid and unambiguous.

## Native engine

Ink renders directly through Vulkan. Naga compiles its shaders to SPIR-V on the build machine; apps do not bundle a runtime shader compiler. The renderer requires Vulkan 1.1 and an sRGB presentation format.

The Android build uses compact RELR relocation tables to reduce native library size.

## Build metadata

A successful bundle emits `.ink/bundle/ink-bundle-v1.json`; only development builds copy it to Android assets. Version 1 contains absolute resolved `inputs`, content-addressed `assets` with source paths, closed `capabilities`, `worker`, `frameworkVersion`, `protocolVersion` and `profile`. Development compilation emits readable development React code and external source maps. Release compilation minifies production code and omits build metadata and source maps. Watching must ignore generated `.ink` inputs and watch graph/config changes; the manifest is local build metadata, not portable source paths.

Development refresh keeps React, the renderer, installed dependencies and device packages in a persistent framework bundle. App modules receive React Refresh registration and hook signatures. Component modules update in place; other modules are cached so task registration and module initialisers do not repeat on every edit. Changes to cached or mixed-export modules change `refreshCompatibilityHash` and request a runtime reload. `devRuntimeHash` identifies the persistent framework. Hook-signature changes remount the affected component. External source maps account for both the framework prefix and per-module transforms. This does not provide general hot replacement of arbitrary module side effects.

The manifest also records rasterised icon variants and their dimensions.
