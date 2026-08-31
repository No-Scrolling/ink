# Performance

Ink targets small Light Phone III applications. Performance is part of the framework interface rather than an implementation detail left to each app.

## Contract

- An unchanged application submits no frames while idle.
- Input and state changes produce at most one rendered update per display frame.
- Fixed-geometry `ForEach` scrolling work is proportional to visible content plus a small overscan window, not the total number of rows. Structurally variable rows use the exact non-virtual fallback.
- Unchanged layout, text, image and GPU data are retained across frames.
- Only native capabilities, permissions, dependencies and assets used by the application enter its APK.
- TypeScript and TSX are compiled ahead of time; applications contain no JavaScript engine.

These guarantees describe release builds. Development builds retain diagnostics and are expected to be larger, but should preserve responsive input and scrolling.

## Budgets

The checked emulator budgets live in [`benchmarks/budgets.json`](../benchmarks/budgets.json), with tighter target-device budgets in [`benchmarks/budgets-lp3.json`](../benchmarks/budgets-lp3.json). They cover APK size, release build time, start-up, memory, threads, fixed-workload CPU and scrolling frame cadence. The verifier selects the LP3 contract for model `TLP301`; the emulator profile is a regression envelope for its host-powered software Vulkan path.

Run the Ink fixtures and verify the current runtime on the Android emulator:

```bash
./benchmarks/measure-ink.sh
```

The full comparison with Expo and Light SDK remains in [`benchmarks/README.md`](../benchmarks/README.md). Measurements use repeated alternating samples and retain raw results. A failed budget is a release regression to investigate, not a threshold to raise until the check passes.

## Light Phone III validation

The emulator is the reproducible regression environment. It cannot establish physical battery drain, thermals, touch-to-present latency or device GPU utilisation.

Renderer scheduling, frame latency, cache sizes and other device-specific tuning must also be measured on the physical TLP301: 1080 × 1240, 480 dpi and 60 Hz. Set `BENCHMARK_DEVICE` to its ADB serial when running the harness. The run is rejected unless Android reports thermal status 0. Perfetto, SurfaceFlinger and Android process accounting provide device evidence; battery claims require external power measurement rather than estimates from an emulator or a short battery-level sample.
