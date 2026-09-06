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

Use `ink` for components, navigation, async hooks, links and text sharing. Add device features through `@ink/*` packages. Native engines follow your imports, so playback-only apps do not include recording or scanning code.

Packages cover audio, auth, background work, barcodes, camera, clipboard, connectivity, downloads, files, LightOS, location, maps, NFC, notifications, secure storage and Store. Networking uses `fetch`, streams and WebSocket. See [Build with Ink](docs/ink.md) for components and package guides.

Import local images, audio and icon collections. You can use JavaScript libraries that support [Ink’s runtime APIs](docs/runtime-compatibility.md).

See [how Ink works](docs/architecture.md) for the engine and [product design](docs/product-design.md) for scope.

The template demonstrates the components and device packages. A [local fixture](examples/light-template/scripts/auth-server.md) supports account and transfer development.

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

Physical Light Phone III measurements from 6 September 2026 using the three matching counter apps. All ARM64 release builds were measured in one interleaved run on the same phone. Values are medians unless stated.

| Counter | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 4.34 MB | 28.72 MB | 10.27 MB |
| Activity launch, median / p95 | 306 / 332 ms | 466 / 587 ms | 1,213 / 1,231 ms |
| Idle memory (PSS) | 17.9 MiB | 59.4 MiB | 21.7 MiB |
| CPU time for 100 taps | 1,420 ms | 4,280 ms | 3,490 ms |
| Clean app build, warm caches | 1.78 s | 58.40 s | 47.87 s |

Each app has a standard header, Public Sans count and Increase action, with the same 100-tap workload. Minor framework rendering differences remain. Clean builds retain compiler and dependency caches. Activity launch is Android's timing, not time to interactive; idle PSS is sampled after two seconds and is not peak memory.

See [the full counter comparison](benchmarks/results/matching-counter-lp3-2026-09-06.md) for raw samples, versions, screenshots, build timings and verified device cleanup.
