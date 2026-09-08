---
title: "Builds, native requirements and assets"
description: "How Ink resolves native requirements, assets and release builds."
---

## Product contract

Include native functionality by explicit package entry point, never by inspecting hook arguments or component names. Import `@ink/audio` for playback and `@ink/audio/microphone` for analysis or `@ink/audio/recording` for recording. Camera capture uses `@ink/camera`; scanning uses `@ink/barcode/scan`. A playback-only app does not require microphone permission or include recorder code, and a capture-only app does not include the barcode scanner.

`@ink/files` provides managed file operations. `@ink/files/images` adds image preparation; `@ink/files/media` adds the photo/video gallery and thumbnail rendering. Basic files and recordings do not require either image feature.

`openURL` and `share` from `ink`, and `@ink/auth`, include the `external` capability for browser and sharing windows. Browser package queries are included only with this capability. The playback service is declared only for `audio-detached`.

Networking follows the same rule: `@ink/network` installs web globals, while `/connectivity` and `/downloads` include only their respective native features. UI imports from `ink` do not install web globals. Development compiler tools live in the SDK workspace’s development dependencies, separate from app runtime dependencies.

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

Import icon collections as `.ink-icons` files containing JSON:

```json
{
  "version": 2,
  "resolution": 52,
  "icons": {
    "settings": "settings",
    "heart": "favorite",
    "heartFilled": { "name": "favorite", "filled": true }
  }
}
```

```tsx
import { settings, heart, heartFilled } from './navigation.ink-icons';
import { Icon, Button } from 'ink';

<Button icon={settings}>Settings</Button>
<Icon name={active ? heartFilled : heart} size={28} />
```

A string declares an outlined icon. Use `{ "name": "favorite", "filled": true }` for a filled icon. The object key becomes its export name. Each reference includes its variant: `Icon`, buttons, tabs and screen actions display the reference you supply. To change appearance at runtime, switch references.

Prefer named imports: unused exports and their raster assets can be removed from release builds. Pass references through props, arrays or objects as ordinary TypeScript values. Both variants are retained when your code can select either one.

For dynamic lookups, use the default collection import and `findIcon(collection, externalName)` from `ink`. It returns `undefined` for an unknown key. A collection used dynamically retains all its possible icons. Unknown Material Symbols names fail the build.

`ink check`, `ink build` and `ink dev` generate `.ink-icons.d.ts` files beside imported collections for TypeScript completion. Ignore these generated declarations in version control. Run `ink check` after adding or changing a collection to refresh its types.

`resolution` sets the raster size in logical units, from 16 to 128. Ink applies its 2.55 pixel scale and native display scaling. Rendering above the declared resolution can soften edges. Back and input-clear icons are always included.

## Native shaders

Ink renders directly through Vulkan. Naga compiles its shaders to SPIR-V on the build machine; apps do not bundle a runtime shader compiler. The renderer requires Vulkan 1.1 and an sRGB presentation format.

## Build metadata

A successful bundle emits `.ink/bundle/ink-bundle-v1.json`; only development builds copy it to Android assets. Version 1 contains absolute resolved `inputs`, content-addressed `assets` with source paths, closed `capabilities`, `worker`, `frameworkVersion`, `protocolVersion` and `profile`. Development compilation emits readable development React code and external source maps. Release compilation minifies production code and omits build metadata and source maps. Watching must ignore generated `.ink` inputs and watch graph/config changes; the manifest is local build metadata, not portable source paths.

Development refresh keeps React, the renderer, installed dependencies and device packages in a persistent framework bundle. App modules receive React Refresh registration and hook signatures. Component modules update in place; other modules are cached so task registration and module initialisers do not repeat on every edit. Changes to cached or mixed-export modules change `refreshCompatibilityHash` and request a runtime reload. `devRuntimeHash` identifies the persistent framework. Hook-signature changes remount the affected component. External source maps account for both the framework prefix and per-module transforms. This does not provide general hot replacement of arbitrary module side effects.

The manifest also records rasterised icon variants and their dimensions.
