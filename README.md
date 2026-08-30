<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./assets/images/title-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="./assets/images/title-light.png">
  <img src="./assets/images/title-light.png" alt="Ink" width="360">
</picture>

<p>An experimental TypeScript framework for small, native Light Phone III apps.</p>

Ink compiles a restricted TypeScript and TSX dialect ahead of time into Rust. Apps use a native Vulkan renderer and do not include a JavaScript runtime.

> [!NOTE]
> Ink is an early experiment and is not ready for production apps.

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

## Features

- TypeScript and TSX authoring
- Ahead-of-time Rust compilation
- Native Vulkan rendering with `wgpu`
- Public Sans and Material Symbols
- Tabs and stack navigation
- Shared and persisted state
- Conditional and repeated UI
- Momentum scrolling
- Lightweight text input and keyboard
- Local images and generated app icons
- Multi-file apps and installed UI packages
- Android development, signing and device tooling through the `ink` CLI

## Modules

- `ink` — UI, layout, navigation, state and persistence
- `@ink/light-sdk` — LightOS integration
- `@ink/network` — HTTP, WebSockets and connectivity (planned)
- `@ink/audio` — Playback and recording (planned)
- `@ink/location` — Device location (planned)
- `@ink/camera` — Camera and code scanning (planned)
- `@ink/nfc` — NFC reading (planned)
- `@ink/notifications` — Local notifications (planned)
- `@ink/background` — Scheduled background work (planned)

## LightOS support

Enable LightOS integration with one import:

```tsx
import "@ink/light-sdk";
```

- [x] Tool discovery
- [x] Service authentication
- [x] SDK version checking
- [x] Haptic preferences
- [ ] Keyboard preferences
- [ ] Permission status
- [ ] Permission requests
- [ ] Device key forwarding
- [ ] Shared files
- [ ] Push registration
- [ ] Push notifications
- [ ] Open dialler
- [ ] Set ringtone

Ink currently targets Light SDK `0.1.1`. Physical Light Phone III builds connect to `com.lightos`. To use the official emulator:

```toml
[light]
server = "com.thelightphone.sdk.emulator"
```

## Development

Check the local toolchain and run the counter example:

```bash
ink doctor
ink -C examples/counter dev
```

## Commands

- `ink check` validates an app.
- `ink dev` builds, installs and watches an app.
- `ink build` creates an optimised, signed APK.
- `ink devices` lists connected Android devices.
- `ink logs` streams app and crash logs.
- `ink info` shows the resolved app and build configuration.
- `ink doctor` checks the development environment.

Use `ink -C <directory> <command>` to run a command for an app outside the current directory.

## Documentation

- [Architecture](docs/architecture.md)
- [Persistence and shared state](docs/adr/0001-persistence-and-shared-state.md)
- [Installed UI packages](docs/adr/0002-installed-source-packages.md)
- [Light SDK adapter](docs/adr/0003-light-sdk-adapter.md)
