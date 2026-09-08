# Vulkan renderer

Ink's Android renderer. `compact.rs` prepares scenes and caches glyphs and images; `gpu.rs` owns Vulkan resources, uploads and presentation through ash. The renderer requires Vulkan 1.1 and an sRGB presentation format.

Naga compiles shaders to SPIR-V on the build machine. Apps do not bundle a runtime shader compiler.

Texture clears and writes share one upload submission and a temporary staging buffer. The batch retains its textures until the upload completes. Rendering returns after submission and presentation, without waiting for GPU completion. Before changing shared buffers or cached textures, the next frame waits for the previous rendering fence. Only one rendering frame is in flight; presentation semaphores belong to individual swapchain images.

With `INK_BENCHMARK=1`, supported devices log GPU command-stream timestamps. These can include acquisition waits and describe the previous completed frame, not input-to-display latency. Normal releases omit these queries.
