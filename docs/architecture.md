# Ink architecture

Ink keeps its public authoring surface small and pushes complexity into a few deep modules.

```text
App.tsx
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

The compiler module hides TSX parsing, restricted-language validation, typed lowering, Rust generation, diagnostics and app branding. Its interface is one build input, a set of generated native artefacts and the native capabilities used by that app. Oxc is an implementation detail behind this seam.

### `crates/ink-core`

The core module owns app state, text focus and editing, route history, logical layout, clipping, scrolling, hit regions and input dispatch. `Screen` centralises the LP3 header, insets, type rhythm and overflow behaviour, `Tabs` owns bottom navigation, and `Navigator` owns a compact stack over compile-time routes. Header titles are centred between equal action slots, independent of the content insets below. Generated applications depend on this small construction interface rather than renderer or Android types.

### `crates/ink-renderer-wgpu`

The renderer module consumes a clipped display list and owns the Vulkan surface, GPU resources, local PNG textures and text/icon caches. `wgpu`, precompiled SPIR-V shaders and the compact Public Sans glyph atlas are implementation details. Rendering is event-driven: a frame is submitted only when state, pointer scrolling, resources or viewport data makes the scene dirty.

### `platform/android`

Android is an adapter. It supplies a surface, lifecycle, pointer events, text edits and system-back requests through a coarse JNI seam. Its internal namespace is fixed while Gradle takes the application ID, name and version from `ink.toml`, so app identity does not leak into Kotlin or native symbol names. The compiler detects `TextInput` and Gradle includes Ink's keyboard source set and resources only for those apps. The adapter builds its `Typeface` from the same static Public Sans bytes used by the renderer, exposed as a direct buffer rather than duplicated as an Android font resource. Rust still owns the text value and focus state.

### `examples`

The counter is the smallest interactive example, exercising one state value and one action on a single screen. The light-template example mirrors the template's top-level tabs and nested settings pages for deterministic visual comparisons while exercising framework-owned screen density, automatic scrolling, tabs, navigation and keyboard text entry. Their public files are `App.tsx` and metadata-only `ink.toml`; generated Rust and Android resources stay in each example's ignored `.ink/` directory.

## Deliberate constraints

- Android only, portrait, API 34+.
- LP3 logical units scale to 2.55 physical pixels at the device's 1080-pixel width, matching the established application density independently of Android display density.
- Public Sans Regular is the default and only bundled font in v0.
- Text is full-opacity, scalar left-to-right with kerning in v0. The supported 24 emoji use compact colour atlases and grapheme-safe editing; complex shaping, wrapping, general font fallback, bold and italic remain future capabilities.
- Material Symbols are rasterised by the compiler. General interface icons use the outlined variant at weight 300 and bottom navigation icons use the filled variant at weight 400; only referenced glyph masks enter generated application code.
- Images are local compile-time PNG assets in v0; remote and Light SDK resources need a resource adapter.
- No JavaScript runtime or arbitrary production npm packages.
- No idle animation loop; redraw only after invalidation or while native scrolling is active.
- Blank black Android splash and first frame.
- First-letter black-and-white launcher artwork generated from app metadata.
- The keyboard is a conditional, Canvas-rendered Android adapter and is absent from apps without `TextInput`; it does not bring Compose into the application. Light SDK remains a future adapter rather than a dependency of the core module.
