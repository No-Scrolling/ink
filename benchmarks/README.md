# Benchmarks

The active comparison contains three counter apps: Ink, Expo (light-template) and Light SDK. Each has a standard header, a centred count starting at zero and an Increase button. Scrolling fixtures have been removed; older list measurements below are historical.

## Recorded results

The [matching-counter LP3 comparison](results/matching-counter-lp3-2026-09-06.md) supplies the current README figures and includes raw runtime/build samples, workload screenshots and verified device cleanup. Earlier comparisons below are historical and used different fixtures.

The earlier [React counter and scrolling results](results/ink-react-lp3-2026-09-06.md) and [Expo and Light SDK results](results/expo-light-sdk-lp3-2026-09-06.md) are retained as historical measurements from before this combined rerun.

A later [lazy image resources and compact native nodes comparison](results/renderer-nodes-lp3-2026-09-06.md) measures **17.84 MiB for the counter and 22.59 MiB for the non-virtualised list**. Compact nodes save approximately 3 MiB on the list; the counter difference is within measurement variation. This targeted Ink follow-up does not replace the full-framework comparison.

The [memory investigation](results/memory-lp3-2026-09-06.md) profiles eager web polyfills, garbage collection, allocator purging and list virtualisation on LP3. These exploratory variants are separate from the published comparison benchmarks.

The [optional web-code split](results/split-web-lp3-2026-09-06.md) saves another 5.16 MiB in the counter and 4.91 MiB in the unchanged non-virtualised list in the LP3 experiment. Release apps now package web code separately and load it automatically on first use. `INK_SPLIT_WEB=0` retains the previous packaging for comparison.

The subsequent [native startup memory reductions](results/native-memory-lp3-2026-09-06.md) bring median idle PSS to **18.07 MiB for the counter and 25.80 MiB for the non-virtualised list**, through deferred native initialisation, fuller startup purging and software rendering of the surrounding Android views. Ink's scene still uses Vulkan. The report includes paired measurements and remaining verification limits.

The [runtime comparison](results/runtime-lp3.md) and [QuickJS-ng / Hermes comparison](results/runtime-engines-lp3.md) record the JavaScript runtime experiments on a physical Light Phone III. Their reports describe the measured builds and limitations.

Apart from the current results and runtime experiments above, files in `results/`, along with `baselines/`, `budgets.json` and `budgets-lp3.json`, are historical measurements or budgets from the removed declarative engine. They do not describe current React app performance. The `ink-updates` results measured that engine's state-to-scene path; its fixture and instrumentation harness have been removed. Check out the recorded source revisions to reproduce those measurements.

## Run current benchmarks

Build and measure the Ink counter with `BENCHMARK_DEVICE=<serial> ./benchmarks/measure-ink.sh`. Set `BENCHMARK_OUTPUT` and `BUILD_BENCHMARK_OUTPUT` to new paths to preserve earlier results.

For all three counters, overlay `apps/expo-counter` on a copy of light-template, preserving its components, hooks, utilities and frozen dependencies. Prepare its ARM64 release APK. Include `apps/light-sdk-counter` as `benchmark-counter` in a temporary Light SDK checkout.

Set `EXPO_COUNTER_DIR` and `LIGHT_SDK_DIR`, then run `./benchmarks/measure-builds.sh`. Clean and no-change builds now measure Counter only. Set `EXPO_COUNTER_APK` and `BENCHMARK_DEVICE` before running `bun benchmarks/measure.ts`. `INK_COUNTER_APK` can select an isolated experiment artifact. No scrolling project or APK is required.

For automatic reservation and cleanup, create a JSON object mapping `ink`, `expo` and `light-sdk` to their absolute APK paths, then run `scripts/agent-tools bench --comparison /absolute/path/counters.json --serial SERIAL --background`. The tool saves raw results and screenshots, removes its installed apps and restores settings. Use a device without existing benchmark installations. Direct harness runs require a manual device reservation and cleanup.

The harness alternates frameworks across 15 cold process launches, five idle-memory samples and five 100-tap workloads. Activity launch is measured with Android `am start -W`; memory uses `dumpsys meminfo`, CPU uses process ticks and frame intervals use SurfaceFlinger. These do not measure time to interactive, peak memory, battery drain or GPU usage. Emulator checks establish appearance and behaviour; published performance measurements use the physical LP3.
