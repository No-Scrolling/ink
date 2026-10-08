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

A React and TypeScript framework to create Light Phone III apps. Powered by QuickJS-ng and Ink's Rust/Vulkan renderer.

> [!IMPORTANT]
> Ink is currently under heavy development.

## Install

```sh
curl -fsSL https://ink.noscroll.ing/install.sh | sh
```

The installer includes the CLI and Bun, then runs `ink setup` in interactive terminals. Setup reuses existing build tools and asks before installing missing components. Follow the printed PATH instructions before running Ink.

Build Android apps on macOS Apple Silicon or Linux x64. Linux requires glibc 2.35 or newer, such as Ubuntu 22.04. Linux ARM64 supports the CLI and JavaScript tooling, but Android builds require an x64 Linux host or Apple Silicon Mac.

```sh
ink create my-app
cd my-app
ink dev
```

Connect a Light Phone III with USB debugging enabled, or start its configured emulator before running `ink dev`. See [Get started](docs/get-started.md) for prerequisites and device setup.

Run `ink update` to install the latest release. Projects synchronise their Ink packages on the next `ink dev`, `ink build` or `ink check`.

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

## Documentation

Start with [Get started](docs/get-started.md), or see [ink.noscroll.ing](https://ink.noscroll.ing).

For release packaging, installation and publishing, see [Releases](docs/releases.md).

## Development

Install workspace dependencies, check the toolchain and run the template:

```bash
bun install
./scripts/ink doctor
./scripts/ink -C examples/light-template dev
```

- [Tuner](examples/tuner): microphone input, adjustable reference pitch and sharp or flat note names.
- [Weather](examples/weather): file-based navigation, tabs and settings.

Commands:

- `ink create <directory>` creates an app using the installed Ink release, or the selected SDK when developing from a checkout.
- `ink lint` checks app source with Oxlint without changing files.
- `ink check` checks formatting, lint, TypeScript and bundling without changing source files. Use `ink format` to apply formatting.
- `ink dev` installs a development host, then transfers bundle generations over ADB. Compatible component edits preserve React state; other JavaScript edits reload the runtime and native changes rebuild the APK. Use `--device <serial>` to select a device.
- `ink build` creates an optimised, signed APK.
- `ink export` renders a route or TSX composition independently of the emulator as a 1080 × 1240 image frame for tldraw. See [Design exports](docs/design-export.md) for setup, fixture data and canvas imports.
- `ink info` shows bundle size, native capabilities and project details.
- `ink devices`, `ink logs` and `ink doctor` help with device setup and debugging.

Import `@ink/lightos` to include LightOS integration.
