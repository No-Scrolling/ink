---
title: "Builds, native requirements and assets"
description: "How Ink resolves native requirements, assets and release builds."
---

## Product contract

Include native functionality by explicit package entry point, never by inspecting hook arguments or component names. Import `@ink/audio` for playback and `@ink/audio/capture` for recording and analysis. Camera capture uses `@ink/camera`; scanning uses `@ink/camera/scan`. A playback-only app does not require microphone permission or include recorder code, and a capture-only app does not include the barcode scanner.

## Declare native requirements

The bundler follows resolved imports, including re-exports, aliases, linked packages and worker code. Renaming an imported component does not change its native requirements. Use `ink info` to inspect the result.

Each supported package ships `ink-native.json` beside `package.json`:

```json
{"version":1,"modules":{"src/index.ts":["audio-detached"],"src/capture.ts":["audio-capture"]}}
```

Paths are relative to the resolved package directory. `*` applies to every loaded module. Named entries apply when that implementation file enters the graph. This supports package export splits without depending on how a consumer names an import. A resolved Ink package missing its declaration fails with an installation/migration diagnostic; unknown capabilities fail during metadata decoding. Ordinary JavaScript dependencies do not require declarations. Third-party packages may provide declarations too.

`crates/ink-compiler/capabilities-v1.json` owns capability costs, dependency closure, permissions and Android source groups. The compiler resolves dependencies after adding explicit `ink.toml` capabilities. Gradle consumes the same catalogue and retains build actions and SDK dependency wiring.

## Import assets

Import local images and MP3 audio. Use HTTPS or managed-file URLs for runtime sources.

```tsx
import photo from './photo.jpg';
import track from './track.mp3';
<Image src={photo} width={200} height={200} />
```

## Import icons

Import icon collections as `.ink-icons` files containing JSON:

```json
{"version":1,"resolution":52,"names":["home","settings"]}
```

```tsx
import icons from './navigation.ink-icons';
import { Icon, Button, Tab, findIcon } from 'ink';
<Icon name={icons.home} size={28} />
<Button icon={icons.settings}>Settings</Button>
<Tab icon={icons.home}>...</Tab>
const optional = findIcon(icons, externalName);
```

Pass imported icon references through props, arrays or objects. `findIcon` returns `undefined` when a name is outside the collection; omit the icon or provide a fallback. Unknown names declared in a collection fail the build. Collections include outlined and filled variants.

`resolution` sets the raster size in logical units, from 16 to 128. Ink applies its 2.55 pixel scale and native display scaling. Rendering above the declared resolution can soften edges. Back and input-clear icons are always included.

## Build metadata

A successful bundle emits `.ink/bundle/ink-bundle-v1.json`; compilation copies it to Android assets. Version 1 contains absolute resolved `inputs`, content-addressed `assets` with source paths, closed `capabilities`, `worker`, `frameworkVersion`, `protocolVersion` and `profile`. Development compilation emits readable development React code and external source maps. Release compilation minifies production code. Watching must ignore generated `.ink` inputs and watch graph/config changes; the manifest is local build metadata, not portable source paths.

Development refresh keeps React, the renderer, installed dependencies and device packages in a persistent framework bundle. App modules receive React Refresh registration and hook signatures. Component modules update in place; other modules are cached so task registration and module initialisers do not repeat on every edit. Changes to cached or mixed-export modules change `refreshCompatibilityHash` and request a runtime reload. `devRuntimeHash` identifies the persistent framework. Hook-signature changes remount the affected component. External source maps account for both the framework prefix and per-module transforms. This does not provide general hot replacement of arbitrary module side effects.

The manifest also records rasterised icon variants and their dimensions.
