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

Please see [ink.noscroll.ing](https://ink.noscroll.ing)

## Development

Install workspace dependencies, check the toolchain and run the template:

```bash
bun install
./scripts/ink doctor
./scripts/ink -C examples/light-template dev
```

For a small app example, see [Tuner](examples/tuner), a chromatic tuner with adjustable reference pitch and sharp or flat note names.

- `ink create <directory>` creates an app outside the repository using the selected local SDK.
- `ink check` checks TypeScript and bundles an app.
- `ink dev` installs a development host, then transfers bundle generations over ADB. Compatible component edits preserve React state; other JavaScript edits reload the runtime and native changes rebuild the APK. Use `--device <serial>` to select a device.
- `ink build` creates an optimised, signed APK.
- `ink info` shows bundle size, native capabilities and project details.
- `ink devices`, `ink logs` and `ink doctor` help with device setup and debugging.

Enable LightOS integration with `[lightos]` and `enabled = true` in `ink.toml`.

## Benchmarks

Physical Light Phone III measurements from 8 September 2026 using the three matching counter apps. All ARM64 release builds were measured in one interleaved run on the same phone. Values are medians unless stated.

| Counter | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| APK size | 1.95 MB | 28.72 MB | 10.27 MB |
| Clean app build, warm caches | 1.77 s | 59.66 s | 48.82 s |
| Activity launch, median / p95 | 222 / 239 ms | 437 / 580 ms | 1,203 / 1,219 ms |
| Idle memory (PSS) | 15.8 MiB | 60.6 MiB | 22.3 MiB |
| CPU time for 100 taps | 1,050 ms | 4,510 ms | 3,630 ms |

Each app has a standard header, Public Sans count and Increase action, with the same 100-tap workload. Minor framework rendering differences remain. Activity launch is Android's timing, not time to interactive; idle PSS is sampled after two seconds and is not peak memory.

Build times are medians of three runs on an Apple M4 Pro, with app outputs cleaned and dependency and compiler caches retained. See [the full counter comparison](benchmarks/results/matching-counter-lp3-2026-09-08.md) for raw samples, versions, screenshots and verified device cleanup.
