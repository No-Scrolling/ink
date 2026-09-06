<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./assets/images/title-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="./assets/images/title-light.png">
  <img src="./assets/images/title-light.png" alt="Ink" width="48">
</picture>
<br><br>
<p>
  <a href="https://github.com/lightphone/light-sdk/tree/3df3c24a21247e70ad59e1bc0393ac6d63840bc2"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/Light%20SDK-0.1.1-e5e5e5?labelColor=5c5c5c"><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/badge/Light%20SDK-0.1.1-black"><img src="https://img.shields.io/badge/Light%20SDK-0.1.1-black" alt="Light SDK 0.1.1"></picture></a>
  <a href="LICENSE"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/badge/licence-MIT-e5e5e5?labelColor=5c5c5c"><source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/badge/licence-MIT-black"><img src="https://img.shields.io/badge/licence-MIT-black" alt="MIT licence"></picture></a>
</p>

A React and TypeScript framework for Light Phone III apps, powered by QuickJS-ng and Ink's Rust/Vulkan renderer.

## Example

```tsx
import { useState } from "react";
import { Button, Screen, Stack, Text } from "ink";

export default function Counter() {
  const [count, setCount] = useState(0);

  return (
    <Screen title="Counter" centered>
      <Stack gap={16} align="center">
        <Text size={40}>Count: {count}</Text>
        <Button onPress={() => setCount(value => value + 1)}>
          Increase
        </Button>
      </Stack>
    </Screen>
  );
}
```

JavaScript runs on a dedicated thread. React batches UI changes into Ink's retained native tree; Rust owns layout, scrolling, text and rendering. Background jobs run in separate headless runtimes.

## Packages

Use `ink` for components, navigation and async hooks. Device APIs live in `@ink/audio`, `@ink/camera`, `@ink/location`, `@ink/nfc`, `@ink/notifications`, `@ink/background`, `@ink/lightos`, `@ink/barcode` and `@ink/clipboard`. Preferences live in `@ink/store`. Networking uses `fetch`, streams and WebSocket.

The build reads native requirements from packages in the resolved module graph and adds explicit capabilities from `ink.toml`. Local images, audio and icon collections are imported assets. JavaScript packages can be bundled when their required runtime APIs are available.

Read [the Ink documentation](docs/ink.md) and [how Ink works](docs/architecture.md). Package pages distinguish implemented features from planned packages.

## Development

Install workspace dependencies, check the toolchain and run the template:

```bash
bun install
./scripts/ink doctor
./scripts/ink -C examples/light-template dev
```

- `ink create <directory>` creates an app outside the repository using the selected local SDK.
- `ink check` checks TypeScript and bundles an app.
- `ink dev` installs a development host, then transfers bundle generations over ADB. Compatible component edits preserve React state; other JavaScript edits reload the runtime and native changes rebuild the APK. Use `--device <serial>` to select a device.
- `ink build` creates an optimised, signed APK.
- `ink info` shows bundle size, native capabilities and project details.
- `ink devices`, `ink logs` and `ink doctor` help with device setup and debugging.

Builds target the LP3's ARM64 ABI. Enable LightOS integration with `[lightos]` and `enabled = true` in `ink.toml`. `ink dev` selects the appropriate host service for the phone or emulator.

See [standalone setup](docs/standalone.md), [development and refresh](docs/development.md), [build declarations and assets](docs/build-contracts.md), [runtime compatibility](docs/runtime-compatibility.md), [native lifetimes](docs/runtime-contracts.md) and [variable-height lists](docs/lists.md).

## Repository

- `packages/`: TypeScript components, hooks and package APIs.
- `crates/ink-runtime/`: QuickJS-ng and its event loop.
- `crates/ink-core/`: retained UI, layout and interaction.
- `crates/ink-renderer-wgpu/`: Vulkan rendering.
- `crates/ink-compiler/` and `crates/ink-cli/`: bundling and development tools.
- `platform/android/`: Android integration, native adapters and APK builds.
- `examples/light-template/`: the reference app.
- `docs/` and `benchmarks/`: documentation and performance measurements.

## Benchmarks

Physical Light Phone III measurements from 6 September 2026. Ink uses the current React/QuickJS-ng engine. Expo and Light SDK were measured in an interleaved run after Ink's run on the same phone. All APKs are release builds targeting ARM64. Values are medians unless stated.

| Counter | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 4.30 MB | 26.32 MB | 10.24 MB |
| Activity launch, median / p95 | 329 / 357 ms | 432 / 558 ms | 1,187 / 1,207 ms |
| Idle memory (PSS) | 35.5 MiB | 54.7 MiB | 18.3 MiB |
| CPU time for 100 taps | 1,400 ms | 4,370 ms | 3,530 ms |
| Clean app build, warm caches | 1.67 s | 56.22 s | 49.04 s |

| 1,000-row scroll | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 4.30 MB | 26.30 MB | 10.33 MB |
| Activity launch, median / p95 | 327 / 366 ms | 1,308 / 1,377 ms | 1,204 / 1,225 ms |
| Idle memory (PSS) | 45.6 MiB | 101.6 MiB | 32.5 MiB |
| CPU time for 12 swipes | 1,490 ms | 2,860 ms | 4,170 ms |
| Clean app build, warm caches | 1.74 s | 55.58 s | 45.14 s |
| Continuous-scroll frame interval, p99 | 16 ms | 16 ms | 16 ms |
| Intervals longer than 17 ms | 1 | 2 | 1 |

The scrolling fixture renders all 1,000 rows without virtualisation. Builds were measured on an Apple M4 Pro Mac; clean steps remove app outputs while retaining compiler, Gradle and dependency caches. Activity launch uses Android's timing, not an instrumented time-to-interactive measurement. The framework-specific fixtures are not pixel-identical.

See [the Ink report](benchmarks/results/ink-react-lp3-2026-09-06.md) and [the Expo/Light SDK report](benchmarks/results/expo-light-sdk-lp3-2026-09-06.md) for raw samples, versions and methodology. Light SDK now packages only ARM64 native libraries; its older multi-architecture APK sizes are not directly comparable.
