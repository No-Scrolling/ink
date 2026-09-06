# Builds, native requirements and assets

The bundler reads the actual resolved modules, including re-exports, aliases and linked packages, for the UI and worker. It does not inspect component names, hook calls or option expressions. Native requirements are conservative: importing audio includes recording, playback and detached playback; importing camera includes capture and scanning. `ink info` reports the resulting native cost.

Each supported package ships `ink-native.json` beside `package.json`:

```json
{"version":1,"modules":{"*":["audio"],"src/player.ts":["audio-playback"]}}
```

Paths are relative to the resolved package directory. `*` applies to every loaded module. Named entries apply when that implementation file enters the graph. This supports package export splits without depending on how a consumer names an import. A resolved Ink package missing its declaration fails with an installation/migration diagnostic; unknown capabilities fail during metadata decoding. Ordinary JavaScript dependencies do not require declarations. Third-party packages may provide declarations too.

`crates/ink-compiler/capabilities-v1.json` owns capability costs, dependency closure, permissions and Android source groups. The compiler resolves dependencies after adding explicit `ink.toml` capabilities. Gradle consumes the same catalogue and retains build actions and SDK dependency wiring.

Import local images and MP3 audio. Local strings in JSX are no longer rewritten. HTTPS and managed-file URLs remain runtime values.

```tsx
import photo from './photo.jpg';
import track from './track.mp3';
<Image src={photo} width={200} height={200} />
```

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

The same branded string reference travels through props and maps. `findIcon` returns `undefined` for names outside the collection; render a fallback or omit the icon. Unknown names inside a collection fail the build. Collections include outlined and filled variants. `resolution` is an integer from 16 to 128 logical units, independent of display size; masks use the existing 2.55 pixel scale and native display scaling. Displaying above the declared resolution may soften edges. Framework back and input-clear icons are always included.

A successful bundle emits `.ink/bundle/ink-bundle-v1.json`; compilation copies it to Android assets. Version 1 contains absolute resolved `inputs`, content-addressed `assets` with source paths, closed `capabilities`, `worker`, `frameworkVersion`, `protocolVersion` and `profile`. Development compilation emits readable development React code and external source maps. Release compilation minifies production code. Watching must ignore generated `.ink` inputs and watch graph/config changes; the manifest is local build metadata, not portable source paths.

Development refresh keeps React, the renderer, installed dependencies and device packages in a persistent framework bundle. App modules receive React Refresh registration and hook signatures. Component modules update in place; other modules are cached so task registration and module initialisers do not repeat on every edit. Changes to cached or mixed-export modules change `refreshCompatibilityHash` and request a runtime reload. `devRuntimeHash` identifies the persistent framework. Hook-signature changes remount the affected component. External source maps account for both the framework prefix and per-module transforms. This does not provide general hot replacement of arbitrary module side effects.

The manifest also records rasterised icon variants and their dimensions. See [verification results](verification-2026-09-06.md) for compiler, refresh and emulator checks.
