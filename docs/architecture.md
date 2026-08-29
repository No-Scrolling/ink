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
ink-renderer-wgpu: surface -> glyph atlas -> GPU frame
   |
   v
Android adapter: lifecycle, SurfaceView and input forwarding
```

## Modules and interfaces

### `crates/ink-cli`

The CLI module is the development interface over project discovery, compilation, Gradle, signing, ADB, Logcat and file watching. Its small command vocabulary keeps those tools behind one seam. Builds are cancellable when watched files change, device identity is always the ADB serial, and routine output stays quiet unless `--verbose` is requested.

### `packages/ink`

This is the app-author interface. It contains compile-time TypeScript declarations for Ink primitives and signals. It has no runtime implementation and contributes no JavaScript to an APK.

### `crates/ink-compiler`

The compiler module hides TSX parsing, restricted-language validation, typed lowering, Rust generation, diagnostics and app branding. Its interface is one build input and a set of generated native artefacts. Oxc is an implementation detail behind this seam.

### `crates/ink-core`

The core module owns app state, logical layout, hit regions and input dispatch. Generated applications depend on its small construction interface rather than renderer or Android types.

### `crates/ink-renderer-wgpu`

The renderer module consumes a display list and owns the Vulkan surface, GPU resources and text cache. `wgpu`, precompiled SPIR-V shaders and the compact Public Sans glyph atlas are implementation details. Rendering is event-driven: a frame is submitted only when state, resources or viewport data makes the scene dirty.

### `platform/android`

Android is an adapter. It supplies a surface, lifecycle and pointer events through a coarse JNI seam. Its internal namespace is fixed while Gradle takes the application ID, name and version from `ink.toml`, so app identity does not leak into Kotlin or native symbol names. It does not own a parallel view hierarchy or application state. The root remains suitable for a future Light Keyboard overlay.

### `examples/counter`

The counter is both the first framework consumer and the current accepted-language boundary. Generated files stay under native build output or clearly marked generated source locations, preserving locality between author code and author-facing diagnostics.

## Deliberate constraints

- Android only, portrait, API 34+.
- A 360dp logical baseline for the 1080×1240 LP3 screen.
- Public Sans Regular is the default and only bundled font in v0.
- Text is scalar left-to-right with kerning in v0; complex shaping, wrapping, fallback, emoji, bold and italic are future capabilities.
- No JavaScript runtime or arbitrary production npm packages.
- No animation loop; redraw only after invalidation.
- Blank black Android splash and first frame.
- First-letter black-and-white launcher artwork generated from app metadata.
- Light SDK and Light Keyboard integration remain future adapters rather than dependencies of the core module.
