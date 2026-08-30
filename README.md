<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./assets/images/title-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="./assets/images/title-light.png">
  <img src="./assets/images/title-light.png" alt="Ink" width="48">
</picture>
<br><br>
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
- Async resources and native actions
- Typed HTTPS data resources
- Conditional and repeated UI
- Momentum scrolling
- Lightweight text input and keyboard
- Local and remote images, plus generated app icons
- Audio playback, recording and microphone analysis
- Foreground device location through LightOS permissions
- Multi-file apps and installed UI packages
- Android development, signing and device tooling through the `ink` CLI

## Modules

- `ink` — UI, layout, navigation, state and persistence
- `@ink/light-sdk` — LightOS integration
- `@ink/network` — typed HTTPS JSON resources
- `@ink/audio` — Playback, recording and microphone analysis
- `@ink/location` — Foreground device location
- `@ink/camera` — Camera and code scanning (planned)
- `@ink/nfc` — NFC reading (planned)
- `@ink/notifications` — Local notifications (planned)
- `@ink/background` — Scheduled background work (planned)

## LightOS support

Enable LightOS integration with one import:

```tsx
import "@ink/light-sdk";
```

Native values are lazy, screen-scoped resources with typed loading, ready and error states:

```tsx
import { lightSdkVersion } from "@ink/light-sdk";
import { Button, Screen, Text } from "ink";

export default function Info() {
  const version = lightSdkVersion();

  return (
    <Screen title="Light SDK">
      {version.status === "loading" ? (
        <Text>Connecting...</Text>
      ) : version.status === "ready" ? (
        <Text>{version.value}</Text>
      ) : (
        <Text>{version.error.message}</Text>
      )}
      <Button onPress={() => version.reload()}>Refresh</Button>
    </Screen>
  );
}
```

Permissions pair a screen-scoped resource with a native LightOS action:

```tsx
import { lightSdkPermission } from "@ink/light-sdk";

const camera = lightSdkPermission("camera");

<Button onPress={() => camera.request()}>Request Camera</Button>
```

The compiler adds the Android permission only when the application uses that resource. LightOS owns the permission prompt, and Ink refreshes the active resource when the application resumes.

- [x] Tool discovery
- [x] Service authentication
- [x] SDK version checking
- [x] Haptic preferences
- [ ] Keyboard preferences
- [x] Permission status
- [x] Permission requests
- [ ] Device key forwarding
- [ ] Shared files
- [ ] Push registration
- [ ] Push notifications
- [ ] Open dialler
- [ ] Set ringtone

Ink currently targets Light SDK `0.1.1`. Physical Light Phone III builds connect to `com.lightos`; `ink dev` selects the official SDK service automatically when its target is an Android emulator.

## Audio

`@ink/audio` provides screen-scoped players, recorders, level meters and pitch detection:

```tsx
import { audioPlayer } from "@ink/audio";

const player = audioPlayer({ usage: "music", playback: "detached" });

<Button onPress={() => player.play({
  src: "./assets/song.mp3",
  title: "Song",
})}>
  Play
</Button>
```

Bundled assets and HTTPS sources use the same player. Detached playback continues through an Android media session after the screen or application leaves the foreground. Apps that do not import `@ink/audio` carry none of its native implementation.

## Location

`@ink/location` combines the LightOS permission screen with a small native Android location adapter:

```tsx
import { currentLocation, locationPermission } from "@ink/location";

const permission = locationPermission();
const location = currentLocation({ accuracy: "precise" });

<Button onPress={() => permission.request()}>Request Location</Button>
```

Recent fixes return immediately; stale fixes trigger a cancellable GPS/network request. Apps that do not import the module carry none of its native implementation.

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
- [Async resources and native actions](docs/adr/0004-async-resources-and-native-actions.md)
- [Audio](docs/audio.md)
- [Location](docs/location.md)
