# Ink architecture

Ink keeps its public authoring surface small and pushes complexity into a few deep modules.

```text
App.tsx + local or installed screen modules + extension imports
   |
   v
ink-compiler: parse -> validate -> typed lowering -> generated Rust
   |
   v
generated app + ink-core: state -> layout -> hit testing -> display list
   |
   v
ink-renderer-wgpu: surface -> glyph/icon atlases + images -> GPU frame
   |
   v
Android adapter: lifecycle, SurfaceView, input and optional native modules
```

## Modules and interfaces

### `crates/ink-cli`

The CLI module is the development interface over project discovery, compilation, conditional native modules, Gradle, signing, ADB, Logcat and file watching. Its small command vocabulary keeps those tools behind one seam. Builds are cancellable when watched files change, device identity is always the ADB serial, and routine output stays quiet unless `--verbose` is requested.

### `packages/ink`

This is the app-author interface. It contains compile-time TypeScript declarations for Ink primitives and signals. It has no runtime implementation and contributes no JavaScript to an APK.

### `crates/ink-compiler`

The compiler module hides package resolution, TSX parsing, the screen-module graph, restricted-language validation, typed lowering, Rust generation, diagnostics and app branding. `App.tsx` is the composition root; local and package-exported `.tsx` modules are zero-argument screens expanded at compile time. Oxc Resolver follows normal `node_modules` lookup and the `ink` export condition behind a framework-owned resolver seam. Local state receives a linked application ID, while matching shared and persisted keys link declarations in separate screens to one slot. Its interface is one build input, a set of generated native artefacts and the native capabilities used by that app.

### `crates/ink-core`

The core module owns app state, persisted-state encoding and hydration, typed async resource state, screen-scoped resource activation, cancellation, single-flight request ordering, text focus and editing, route history, logical layout, clipping, scrolling, hit regions and input dispatch. Resource reads, one-shot native actions and cancellation are distinct request kinds. `Screen` centralises the LP3 header, insets, type rhythm and overflow behaviour, `Tabs` owns bottom navigation, and `Navigator` owns a compact stack over compile-time routes. Header titles are centred between equal action slots, independent of the content insets below. Generated applications depend on this small construction interface rather than renderer or Android types.

### `crates/ink-renderer-wgpu`

The renderer module consumes a clipped display list and owns the Vulkan surface, GPU resources, local PNG textures and text/icon caches. `wgpu`, precompiled SPIR-V shaders and the compact Public Sans glyph atlas are implementation details. Rendering is event-driven: a frame is submitted only when state, pointer scrolling, resources or viewport data makes the scene dirty.

### `platform/android`

Android is an adapter. It supplies a surface, lifecycle, pointer events, text edits, system-back requests, app-private persistence and native request execution through a coarse JNI seam. It enforces operation timeouts and forwards cancellations into native adapters. Native work completes on Android's UI thread; Rust validates tagged results, rejects stale completions and rebuilds the scene. Its internal namespace is fixed while Gradle takes the application ID, name and version from `ink.toml`, so app identity does not leak into Kotlin or native symbol names. The compiler detects text input, native extensions and permissions independently, and Gradle includes only the source sets and manifest capabilities used by an application. The keyboard builds its `Typeface` from the same static Public Sans bytes used by the renderer, exposed as a direct buffer rather than duplicated as an Android font resource. The Light SDK adapter owns service discovery, Binder authentication, cancellation, permission activity hand-off, protocol version checks and preference translation without pulling Compose into the app. The audio adapter owns Media3 playback, media sessions, encoded recordings and raw microphone capture; Rust owns realtime analysis and controller state. Core remains responsible for resource state, text values, focus state and persistence.

### `examples`

The counter is the smallest interactive example, exercising one state value and one action on a single screen. The light-template example keeps navigation composition in `App.tsx` and places each tab or nested page in a local screen module, while mirroring the template for deterministic visual comparisons. It imports the first-party Light SDK extension to exercise conditional native integration. Generated Rust and Android resources stay in each example's ignored `.ink/` directory.

## Deliberate constraints

- Android only, portrait, API 34+.
- LP3 logical units scale to 2.55 physical pixels at the device's 1080-pixel width, matching the established application density independently of Android display density.
- Public Sans Regular is the default and only bundled font in v0.
- Text is full-opacity, scalar left-to-right with kerning in v0. The supported 24 emoji use compact colour atlases and grapheme-safe editing; complex shaping, wrapping, general font fallback, bold and italic remain future capabilities.
- Material Symbols are rasterised by the compiler. General interface icons use the outlined variant at weight 300 and bottom navigation icons use the filled variant at weight 400; only referenced glyph masks enter generated application code.
- Images support compile-time PNG assets and HTTPS resources. Remote bytes are bounded, decoded off the render path and cached for the process lifetime.
- No JavaScript runtime or arbitrary JavaScript packages. Installed Ink source packages are compiled under the same restricted language as application screens.
- No idle animation loop; redraw only after invalidation or while native scrolling is active.
- Blank black Android splash and first frame.
- First-letter black-and-white launcher artwork generated from app metadata.
- The keyboard is a conditional, Canvas-rendered Android adapter and is absent from apps without `TextInput`; it does not bring Compose into the application.
- Light SDK support is another conditional Android adapter. It is absent without `@ink/light-sdk`, pins one upstream protocol version and does not become a dependency of the core module.
- Audio is conditional. Media3 enters an APK only when a player is declared, its session module only for detached playback, and microphone capture and analysis only with `@ink/audio`.
