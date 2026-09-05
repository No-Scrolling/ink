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

The build discovers supported native integrations from imports and explicit capabilities in `ink.toml`. JavaScript packages can be bundled when their required runtime APIs are available.

Read [the Ink documentation](docs/ink.md) and [how Ink works](docs/architecture.md). Package pages distinguish implemented features from planned packages.

## Development

Install workspace dependencies, check the toolchain and run the template:

```bash
bun install
./scripts/ink doctor
./scripts/ink -C examples/light-template dev
```

- `ink check` checks TypeScript and bundles an app.
- `ink dev` builds, installs and watches an app; use `--device <serial>` to select a device.
- `ink build` creates an optimised, signed APK.
- `ink info` shows bundle size, native capabilities and project details.
- `ink devices`, `ink logs` and `ink doctor` help with device setup and debugging.

Builds target the LP3's ARM64 ABI. Enable LightOS integration with `[lightos]` and `enabled = true` in `ink.toml`. `ink dev` selects the appropriate host service for the phone or emulator.

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

[Recorded LP3 measurements](benchmarks/README.md) cover the earlier declarative engine. The [QuickJS-ng/Hermes comparison](benchmarks/results/runtime-engines-lp3.md) measures a small synchronous counter embedding. Neither measures the complete React runtime; see [the scope of those results](docs/architecture.md#lp3-runtime-benchmark).
