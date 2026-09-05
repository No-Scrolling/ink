# LP3: QuickJS-ng versus Hermes

Measured 2026-09-04T20:33:52.749Z on the same physical Light Phone III (TLP301), Android 14 / API 34, arm64-v8a, 60 Hz. All four variants were interleaved in this run and contain no Effect.

QuickJS-ng remains the stronger choice for this counter workload's footprint. Hermes bytecode saves 4.2 µs on the median JavaScript round trip, while the median complete state/UI update is effectively identical at 0.301 ms. This Hermes embedding adds 3.49 MB of APK size and 1.70 MiB of idle PSS relative to QuickJS-ng. This is evidence about a tiny synchronous interaction, not a general ranking of JavaScript engines.

| Metric | Native Ink | QuickJS-ng source | Hermes source | Hermes bytecode |
| --- | ---: | ---: | ---: | ---: |
| APK size | 3.01 MB | 3.70 MB | 7.19 MB | 7.19 MB |
| Cold start, median | 316 ms | 313 ms | 323 ms | 327 ms |
| Cold start, p95 | 335 ms | 327 ms | 351 ms | 359 ms |
| Idle PSS, median | 23.88 MiB | 24.54 MiB | 27.12 MiB | 26.24 MiB |
| Runtime initialisation, median | 0.000 ms | 0.687 ms | 4.641 ms | 3.997 ms |
| JavaScript round trip, median | 0.0 µs | 21.8 µs | 21.1 µs | 17.7 µs |
| JavaScript round trip, p95 | 0.0 µs | 30.4 µs | 30.0 µs | 24.8 µs |
| State/UI update, median | 0.274 ms | 0.301 ms | 0.302 ms | 0.301 ms |
| State/UI update, p95 | 0.538 ms | 0.564 ms | 0.571 ms | 0.576 ms |
| Renderer CPU wall time, median | 5.030 ms | 5.043 ms | 5.000 ms | 4.933 ms |
| CPU for 100 taps, median | 920 ms | 930 ms | 930 ms | 910 ms |

## What ran

The existing counter App.tsx and native Vulkan renderer are unchanged. Rust owns the counter state and scene updates. The JavaScript variants replace only the arithmetic of the increment action. Both engine adapters retain a runtime and cached function for the engine's lifetime, and run synchronously on the calling UI thread.

- QuickJS-ng **0.15.1**, vendored by rquickjs **0.12.2**, evaluates the 82-byte minified plain-JavaScript bundle at startup. It is built through the existing Rust release configuration, which prioritises size.
- Hermes **250829098.0.17** uses the [official Android release AAR](https://repo.maven.apache.org/maven2/com/facebook/hermes/hermes-android/250829098.0.17/), with matching JSI source from [React Native 0.87.1](https://github.com/facebook/react-native/tree/v0.87.1/ReactCommon/jsi). React Native's renderer and app framework are not included. JSI is compiled with -O3; the adapter uses Ink's release settings. The stock engine binary is not rebuilt locally.
- Hermes source evaluates the identical 82-byte JavaScript bundle. Hermes bytecode runs that bundle precompiled with the matching hermes-compiler package, -O, and HBC version 98. This is interpreted bytecode, not native machine code. QuickJS bytecode was not measured.
- Hermes uses its default interpreter configuration, with JIT disabled. Intl is disabled because this standalone embedding does not initialise its Java bindings. The benchmark does not exercise async hosting or package compatibility.

Hermes's packaged native libraries are:

| Library | Stored size |
| --- | ---: |
| libc++_shared.so | 1.374 MB |
| libfbjni.so | 0.177 MB |
| libhermesvm.so | 2.477 MB |
| libink_android.so | 2.944 MB |
| libjsi.so | 0.119 MB |

The Hermes figure includes its required JSI, fbjni and C++ shared libraries. Native libraries are stored without ZIP compression in these signed APKs. This is the cost of the specific stock-AAR embedding; a custom standalone Hermes build could change it. It is not an irreducible engine-size claim.

## Method and validation

The existing harness alternated variant order over 15 cold starts, five idle-memory samples taken two seconds after launch, and five workloads of 100 ADB taps per variant. All **2,000 updates** passed the checks for sequential values 1–100, native scene updates and the correct embedded revision. APK hashes and every recorded source hash were checked before measurement. Runtime-initialisation samples were present in every JavaScript workload.

The build marker is `ec1358841dc6-b109ffee222b`. [Raw results](runtime-engines-lp3.json) contain every sample, APK hashes, source hashes, download URLs, archive hashes, prepared-library hashes and compiler details. Thermal status was 0 before and 0 after the run. The harness restored temporary animation and keep-awake settings.

An initial Hermes smoke launch failed because Android resolved a dependency's Java initialiser. The adapter now provides Ink's own JNI load entry point. That failed build was excluded; all four measured APKs were rebuilt from the same final source snapshot.

## Interpretation

Hermes bytecode's slightly faster function call did not produce a meaningful difference in the complete state/UI update interval. The CPU totals also overlap across runs and use coarse 10 ms process-accounting ticks; the 910 versus 930 ms medians do not establish a battery-efficiency advantage.

Bytecode reduced Hermes's measured runtime-initialisation interval from 4.641 ms to 3.997 ms. Its whole-app cold-start median was nevertheless slightly higher than the source variant. Launch-time distributions overlap, and native Ink also had a slightly higher median than QuickJS-ng. These small whole-launch differences should not be interpreted as a causal speed-up or slowdown from adding an engine or bytecode.

Idle PSS includes Android, rendering and native-library memory, not just the JavaScript heap. Baseline PSS differs from earlier runs, so use the fresh interleaved baseline here rather than subtracting numbers from the earlier Effect comparison.

The JavaScript interval includes the Rust/engine adapter and number conversion. The state/UI interval includes pointer-up handling, the script call and scene work. Renderer CPU wall time is separate. None is end-to-end input-to-photon latency or GPU duration. The ADB tap workload includes deliberate gaps and is not a throughput benchmark.

For Ink's current pattern of native rendering with small JavaScript actions, these results favour QuickJS-ng's footprint. Choosing an engine for larger apps still requires representative parsing, filtering, allocation, asynchronous I/O and responsiveness measurements. This counter does not measure sustained computation, garbage-collection pressure, scrolling under concurrent updates or battery drain.

The experimental counter adapters and build scripts have been retired. These historical results retain the original samples and source hashes; the current framework uses its dedicated React / QuickJS-ng runtime.
