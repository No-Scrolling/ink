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

A TypeScript framework for small, fast Light Phone III apps.

Write apps in TypeScript and TSX. Ink builds them as native Android apps powered by Rust and Vulkan, without bundling a JavaScript engine.

## Example

```tsx
import { Button, Screen, Stack, Text, state } from "ink";

export default function Counter() {
  const count = state(0);

  return (
    <Screen title="Counter" centered>
      <Stack gap={16} align="center">
        <Text size={40}>Count: {count.value}</Text>
        <Button onPress={() => count.set(count.value + 1)}>
          Increase
        </Button>
      </Stack>
    </Screen>
  );
}
```

## Capabilities

Ink keeps everyday UI and state in its core package. Device features live in optional modules, and apps only include the native code for modules they import.

| Capability | Import | What it provides | Docs |
| --- | --- | --- | --- |
| UI and navigation | `ink` | Screens, stacks, tabs, routes, conditional UI, repeated UI, and momentum scrolling. | [Core Ink](docs/ink.md) |
| State and async work | `ink` | Shared, persisted, and computed state; typed route data; async resources and native actions. | [Core Ink](docs/ink.md), [Data and effects](docs/data.md) |
| Text, input and images | `ink` | Public Sans, system emoji, Material Symbols, text input, the Ink keyboard, local and remote images, and generated app icons. | [Core Ink](docs/ink.md) |
| LightOS | `@ink/light-sdk` | Preferences, hardware keys, permissions, dialler, ringtone, and UnifiedPush. | [LightOS](docs/light-sdk.md) |
| Networking | `@ink/network` | Typed HTTPS reads, cached resources, and mutations. | [Data and effects](docs/data.md) |
| Notifications | `@ink/notifications` | Local reminders, notification permission, UnifiedPush delivery, and notification taps. | [Notifications](docs/notifications.md) |
| Background work | `@ink/background` | Typed, persisted periodic network resources. | [Background work](docs/background.md) |
| Audio | `@ink/audio` | Local and remote playback, recording, level metering, and microphone analysis. | [Audio](docs/audio.md) |
| Location | `@ink/location` | Foreground location through the LightOS permission flow. | [Location](docs/location.md) |
| Camera | `@ink/camera` | An Ink camera preview, photo capture, and QR and barcode scanning. | [Camera](docs/camera.md) |
| NFC | `@ink/nfc` | Screen-scoped NFC tag and NDEF reading. | [NFC](docs/nfc.md) |

Ink supports multi-file apps and installed UI packages. Read [how Ink builds and runs an app](docs/architecture.md) for the underlying model.

### LightOS support

Enable LightOS integration with one import:

```tsx
import "@ink/light-sdk";
```

Ink targets Light SDK `0.1.1`. Apps connect to `com.lightos` on a Light Phone III, while `ink dev` automatically uses the official SDK service on an Android emulator.

Supported LightOS features:

- [x] Tool discovery
- [x] Service authentication
- [x] SDK version checking
- [x] Haptic preferences
- [x] Keyboard preferences
- [x] Permission status and requests
- [x] Device key forwarding
- [x] Secure ringtone file hand-off
- [x] UnifiedPush registration and delivery
- [x] Background message notifications
- [x] Open dialler
- [x] Set ringtone

## Benchmarks

Six equivalent arm64 release apps were measured on the same physical Light Phone III. Values are medians; cold starts also show the 95th percentile. Build times use the same development computer.

| Counter | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| App file size | **3.07 MB** | 32.00 MB | 25.83 MB |
| Clean release build | **1.29 s** | 36.20 s | 47.92 s |
| Cold start, typical / slow | **296 / 372 ms** | 516 / 591 ms | 1,186 / 1,226 ms |
| Active memory while idle | **17.9 MiB** | 66.6 MiB | 18.1 MiB |
| CPU time for 100 taps | **860 ms** | 4,300 ms | 3,650 ms |

| 1,000-row scroll | Ink | Expo | Light SDK |
| --- | ---: | ---: | ---: |
| App file size | **3.07 MB** | 31.98 MB | 25.93 MB |
| Active memory while idle | **18.7 MiB** | 116.6 MiB | 32.3 MiB |
| CPU time for 12 swipes | **1,090 ms** | 2,960 ms | 4,040 ms |
| 99% of frame intervals | **≤16 ms** | **≤16 ms** | **≤16 ms** |
| Intervals longer than 17 ms | **0** | 1 | **0** |

Ink leads on app size, build time, startup, memory, and CPU use. See the [full methodology and physical-device results](benchmarks/README.md).

## Development

Check the local toolchain and run the counter example:

```bash
ink doctor
ink -C examples/counter dev
```

- `ink check` validates an app.
- `ink dev` builds, installs and watches an app.
- `ink build` creates an optimised, signed APK.
- `ink devices` lists connected Android devices.
- `ink logs` streams app and crash logs.
- `ink logs --resources` shows native request transitions and timings.
- `ink info` explains retained rendering paths, module capabilities, native cost and app data size.
- `ink doctor` checks the development environment.

Use `ink -C <directory> <command>` to run a command for an app outside the current directory.
