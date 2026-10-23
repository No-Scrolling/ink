# Extended performance coverage

`bench:coverage` measures app journeys, renderer work and Android media/lifecycle workloads through the production compiler, runtime and native engines.

This focused suite also forms part of [`bun run bench`](README.md), which runs all 77 current-build host and renderer workloads. The full command includes existing renderer cases once, together with these new cases. Android is added with `--serial` in either command.

The [23 October 2026 baseline](coverage-results-2026-10-23.md) records the first complete run and its measurement limits.

```sh
bun run bench:coverage --serial emulator-5556
```

Use the Light Phone III emulator started with `emulator -avd Light_Phone_III -writable-system`. Choose its actual serial. A physical device is required for meaningful battery or energy measurements; emulator battery values are not energy evidence.

The default run uses three fresh host processes per group, 20 requested samples, four discarded warm-ups and a 120-second sustained Android workload. Renderer workloads apply their own minimum frame counts and cold-renderer caps. The raw results record the actual counts.

Android runs accept 2–48 samples and 120–3,600 sustained seconds. Host-only runs accept up to 1,000 samples. The device timeout includes the requested sustained period, cold/image/lifecycle observations and idle collection.

## Build and measure separately

```sh
bun run bench:coverage --phase build --output .test-output/coverage/baseline --serial emulator-5556
bun run bench:coverage --phase run --output .test-output/coverage/baseline --serial emulator-5556
```

Builds finish before measurements. The run phase verifies saved executable and fixture hashes and rejects active compilation. Host journey and renderer groups alternate order between rounds. Omit `--serial` for host-only runs; these do not establish Android coverage.

## Workloads and endpoints

| Area | Observation |
| --- | --- |
| Typing and search | Native editor input through final correct list publication and ready visible rows. Local joined-field filtering exercises the automatic helper; database search exercises real queries, cancellation and arrivals. Pending labels alone do not finish a sample. |
| Structural lists | Insert, delete, reverse and change row geometry at sizes below and above automatic native-storage thresholds. Check keys, current content, viewport readiness and bounded mounted hosts. |
| Persisted startup | Write through the public store API, exit the seed process and reopen in a fresh process/runtime. Validate saved records and the final usable scene. Filesystem caches are retained. |
| Cold text and atlas turnover | Fresh renderer caches, explicit per-script font coverage and alternating glyph sets. Reload saved PNGs and check affected regions and cache consistency. Unsupported host glyphs are reported. |
| Image loading and scrolling | Android assets, managed files and deterministic local HTTP images through their real loaders. Observe submitted frames and validate saved pixels. Renderer-only decoded-image turnover remains a separate workload. |
| Navigation and resource states | Loading, failure, retry, ready content and back navigation through real routing and resources. Verify retained state and release/cancellation endpoints. |
| Sustained usage and idle | Android process memory and CPU before, during and after sustained work, followed by settled idle. Physical energy-counter availability is recorded separately. |
| Large uploads | Sustained full-frame and changing-size uploads. Record actual staging allocations, Vulkan allocation bytes, payload bytes and retained capacities. Validate every frame against authored colours. |

## Timing boundaries

Host journey endpoints are native scene application, not Android display presentation. Search observers record real preparation completion and cache use without replacing the native work.

Renderer `cpu_ns` ends at submission. GPU timestamps, when available, cover the renderer command buffer including offscreen readback and exclude the preceding upload command buffer. Completion/readback wall time is separate. Renderer construction, fixture generation, PNG writes and pixel assertions are outside render samples. Cold-renderer construction is recorded separately; process-wide font parsing may already be warm.

Android frame endpoints are submission unless a workload explicitly reports a supported presentation endpoint. A SurfaceView's `gfxinfo` or FrameTimeline output does not establish physical input-to-display latency. Local HTTP timing includes the fixture server and emulator transport.

Memory figures name their scope: process PSS/RSS, renderer texture/buffer capacity and staging allocations measure different resources. They are not interchangeable. Emulator CPU and memory observations do not establish hardware power use.

## Evidence

Each output directory retains compiled assets, executable hashes, source snapshots and hashes, raw process output, per-operation samples, correctness observations and a report. Renderer endpoints include PNGs reloaded for pixel checks. Android output includes its APK/build metadata, logs and device observations. The runner reports errors rather than silently omitting failed workloads.

These runs establish baselines for the new workloads. They do not claim an optimisation or a before/after speed improvement. Preserve an output directory before replaying it; replay overwrites its run results. The saved executable/assets permit replay, while source hashes and the tracked patch identify the measured checkout.
