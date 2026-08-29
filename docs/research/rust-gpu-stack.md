# Rust GPU stack for Ink

Research date: 29 August 2026. This note evaluates `wgpu` + `cosmic-text` + `swash` for an Android-only LP3 renderer, against a small Android Canvas adapter and a CPU renderer built with `tiny-skia`.

## Recommendation

Do not put `wgpu` on the first counter's critical path. Prove Ink's TypeScript-to-Rust compiler, state model, layout, hit testing and Android packaging through the Android Canvas adapter described in [android-rust-prototype.md](./android-rust-prototype.md). Keep the Rust display-list interface renderer-neutral, then make `wgpu` an explicit second renderer experiment.

`wgpu` is technically credible for an eventual fully Rust-owned renderer. It has first-class Vulkan support on Android, and its Android surface can be built from the same `ANativeWindow` that a thin Kotlin `SurfaceView` host exposes. The objection is proportionality: a static counter needs none of the abstraction's cross-platform GPU leverage, while it would immediately incur GPU initialisation, shader/pipeline management, surface lifecycle handling, glyph-atlas code and a material binary/driver-risk budget.

If Rust-owned pixels are non-negotiable for the first slice, prefer `tiny-skia` + `cosmic-text`/`swash` over `wgpu`. That is a much smaller experiment, but it is CPU-only and needs a full pixel buffer. It should therefore remain behind the same renderer seam.

## Android and surface viability

- `wgpu` 30 documents Vulkan as first-class on Linux/Android; GLES 3 is only downlevel/best-effort. For LP3, compile only Vulkan unless physical-device verification demonstrates a need for a GLES fallback ([wgpu supported platforms](https://github.com/gfx-rs/wgpu#supported-platforms)).
- Android ships `libvulkan` from API 24, but an app must still enumerate at runtime because the device GPU may not support Vulkan. API 34 or 36 alone is therefore not the meaningful compatibility test; the LP3 GPU/driver is ([Android Vulkan/NDK API](https://developer.android.com/ndk/guides/stable_apis#vulkan)).
- A normal Kotlin `Activity` can own a `SurfaceView`. Its `SurfaceHolder.Callback` can pass the Java `Surface` to one JNI attach call, forward resize/destroy events, and leave all frames inside Rust. `ANativeWindow_fromSurface` acquires the corresponding native window and requires a matching release ([Android native-window JNI reference](https://developer.android.com/ndk/reference/group/native-activity#anativewindow_fromsurface)).
- The Rust `ndk` crate's `NativeWindow::from_surface` wraps that operation, releases it on drop and implements `raw-window-handle` 0.6's `HasWindowHandle`; this matches `wgpu::Instance::create_surface` directly ([`ndk::NativeWindow` source](https://docs.rs/ndk/latest/src/ndk/native_window.rs.html), [`wgpu::Instance::create_surface`](https://docs.rs/wgpu/latest/src/wgpu/api/instance.rs.html#175-259)).
- Treat `surfaceCreated`/`surfaceDestroyed` as a strict ownership boundary. `wgpu` requires the underlying window handle to outlive its `Surface`, so Rust must drop the `wgpu::Surface` before releasing/replacing its `NativeWindow` ([wgpu surface safety contract](https://docs.rs/wgpu/latest/wgpu/enum.SurfaceTargetUnsafe.html#variant.RawHandle)).
- A thin Kotlin host is preferable to `NativeActivity` here. It keeps the future Light keyboard, accessibility, blank system splash and ordinary Android lifecycle native to the platform. Android recommends `GameActivity` rather than `NativeActivity` for new native games, but Ink is not a game loop and does not need that extra AAR or native-app-glue layer ([GameActivity guidance](https://developer.android.com/games/agdk/game-activity/get-started)).

The JNI seam should carry lifecycle/input events and one renderer attachment, not draw calls. Android explicitly says to minimise both marshalling volume and call frequency ([JNI guidance](https://developer.android.com/ndk/guides/jni-tips#general-tips)).

## What the text stack does — and does not do

`cosmic-text` covers font matching/fallback, shaping, bidirectional text, wrapping/layout and editing. It uses HarfRust for shaping and optionally uses `swash` for glyph rasterisation. A `Buffer` produces positioned layout runs; `SwashCache` produces rasterised glyph images ([cosmic-text overview](https://docs.rs/cosmic-text/latest/cosmic_text/), [`Buffer`](https://docs.rs/cosmic-text/latest/cosmic_text/struct.Buffer.html), [`SwashCache`](https://docs.rs/cosmic-text/latest/cosmic_text/struct.SwashCache.html)).

It does not render a `wgpu` frame. Ink would still need to:

1. allocate and evict a glyph atlas;
2. rasterise missing glyphs with `swash` on the CPU;
3. upload masks/colour glyphs to GPU textures;
4. batch glyph quads and clipping into render passes;
5. invalidate shaped buffers and atlas entries when text, font size or scale changes.

`SwashCache` itself stores rasterised images and outlines in hash maps with no documented budget/eviction interface, so Ink should not use it as an unbounded lifetime cache for arbitrary application text ([`SwashCache` fields](https://docs.rs/cosmic-text/latest/cosmic_text/struct.SwashCache.html)).

Public Sans makes the first slice simpler and more deterministic. Do not call `FontSystem::new()` on Android: upstream warns it may take up to a second in release, and its underlying `fontdb::load_system_fonts` has no Android implementation. Construct a database from the bundled TTF bytes and use `FontSystem::new_with_fonts` or `new_with_locale_and_db` instead ([`FontSystem` initialisation](https://docs.rs/cosmic-text/latest/cosmic_text/struct.FontSystem.html), [`fontdb` platform source](https://docs.rs/fontdb/latest/src/fontdb/lib.rs.html#391-478)). The current template's Public Sans Regular asset is only 60,816 bytes, measured locally.

For the counter, `Shaping::Basic` is appropriate because Ink controls the Latin text and font. It is explicitly cheaper but does not handle complex scripts or font fallback; external/user text later requires `Advanced` shaping and an intentional Android fallback-font strategy ([cosmic-text shaping modes](https://docs.rs/cosmic-text/latest/src/cosmic_text/shape.rs.html#26-43)).

## Size, memory and performance

There is no trustworthy upstream number for a feature-trimmed Ink APK, so size must be measured on the actual `arm64-v8a` release build.

- `wgpu`'s defaults enable every desktop/mobile backend plus WGSL. For Android, use `default-features = false` and opt into only `std`, `vulkan` and the chosen shader input. The current crate exposes these independently, which makes trimming possible ([wgpu feature definitions](https://github.com/gfx-rs/wgpu/blob/trunk/wgpu/Cargo.toml#L793-L1028)).
- This is still not a tiny abstraction: `wgpu` includes resource tracking, validation and shader translation. The current generic `wgpu-native` Android release archive is around 15 MB per ABI, although that C API distribution is not representative of a statically feature-trimmed Ink `libink.so`; it is only a warning not to assume the cost is negligible ([wgpu-native releases](https://github.com/gfx-rs/wgpu-native/releases)).
- The latest `wgpu` requires Rust 1.87. Ink's current default toolchain is 1.85.0, so using current `wgpu` requires a pinned toolchain upgrade (the locally installed 2026 nightly is sufficiently new) ([wgpu MSRV](https://github.com/gfx-rs/wgpu#msrv-policy)).
- GPU memory includes the Android surface image queue plus Ink's glyph atlas, decoded image textures, staging buffers and any intermediate render targets. The simple UI should render only when invalidated; no animation means there is no reason to submit continuously. Exact resident memory is device/driver dependent and must be profiled on LP3 rather than inferred from desktop benchmarks.
- Driver variance is real even on current releases: wgpu 28.0.1 included a fix for a crash on some Mali Android drivers. Physical LP3 verification is mandatory before selecting it as the default renderer ([wgpu 28.0.1 release](https://github.com/gfx-rs/wgpu/releases/tag/v28.0.1)).

For images, `wgpu` only supplies textures and copies. Ink still needs image decoding, dimension-aware downsampling, texture caching and eviction. Decoding every asset at source resolution would erase the memory advantage of a small UI even if the renderer itself is efficient.

## Alternatives

### Android Canvas adapter — best first counter

A single custom `View` receives a hardware-accelerated Canvas by default and can draw text, bitmaps and simple shapes. Android records hardware-accelerated view drawing into display lists, and only dirty views need their display lists updated ([Android hardware drawing model](https://developer.android.com/topic/performance/hardware-accel#drawing-models)). This aligns closely with Ink's static, event-driven UI.

Advantages are the smallest new native surface area, mature platform text/image rendering, straightforward Public Sans loading, and direct compatibility with accessibility and the Light keyboard. The cost is that Kotlin/Android executes the display list rather than Rust owning pixels. Keep Rust responsible for state, layout, hit testing and display-list generation so this remains an adapter rather than a second UI model.

Do not issue one JNI call per primitive. Transfer a compact display list through a reusable direct buffer, then execute it in `onDraw`.

### `tiny-skia` + `cosmic-text`/`swash` — smallest Rust-owned renderer

`tiny-skia` is an intentionally minimal CPU-only 2D rasteriser. Upstream reports roughly 200 KiB added binary size, supports AArch64 NEON, and deliberately excludes text, so `cosmic-text`/`swash` remains necessary ([tiny-skia README](https://github.com/linebender/tiny-skia)).

At the configured LP3 emulator resolution of 1080 × 1240, one packed RGBA buffer is 5,356,800 bytes (about 5.11 MiB), before the Android surface's own buffer queue and glyph/image caches. `tiny-skia::Pixmap` owns premultiplied RGBA pixels and has no external stride, so drawing into a separate packed pixmap and copying to a strided `ANativeWindow` buffer is the straightforward implementation ([`Pixmap`](https://docs.rs/tiny-skia/latest/tiny_skia/struct.Pixmap.html)). That extra full-screen buffer is a concrete memory/bandwidth trade-off.

For event-driven text/image screens this may still be entirely acceptable, and it avoids Vulkan driver and shader complexity. Benchmark one full redraw, tap-to-frame latency and resident memory on LP3 before committing.

## A useful renderer seam

Keep the deep Rust engine independent of these choices:

```text
compiled app -> state/update -> layout + hit tree -> display list
                                                   |
                       +---------------------------+------------------+
                       |                           |                  |
                 Android Canvas              tiny-skia             wgpu
                    adapter                 CPU adapter         Vulkan adapter
```

The interface should accept a stable display list plus viewport/resources and expose attach, detach and render-on-dirty operations. Surface lifecycle, GPU/Canvas objects, atlases and buffers stay inside each adapter implementation. This seam lets the counter answer the important compiler/runtime questions without making the most expensive renderer choice irreversible.

## Go/no-go criteria for the `wgpu` follow-up

Adopt `wgpu` as Ink's default renderer only if the Vulkan spike on the physical LP3 demonstrates all of the following against the Canvas baseline:

- reliable surface create/destroy/resume across repeated background/foreground cycles;
- first visible frame without a noticeable blank interval after the system splash;
- acceptable release APK and `libink.so` size with one ABI and trimmed features;
- lower or otherwise justified resident memory for representative text/image screens;
- correct Public Sans rasterisation at LP3 scale;
- no continuous rendering or idle CPU/GPU work;
- a clear feature that benefits from GPU ownership enough to repay the added implementation depth.

Until then, `wgpu` is a promising renderer module, not a prerequisite for Ink itself.
