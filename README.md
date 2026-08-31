<picture>
  <source media="(prefers-color-scheme: dark)" srcset="./assets/images/title-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="./assets/images/title-light.png">
  <img src="./assets/images/title-light.png" alt="Ink" width="48">
</picture>
<br><br>
<p>A TypeScript framework for performant apps for the Light Phone III.</p>

Ink compiles TypeScript into a small native Android app at build time, powered by Rust and Vulkan. There is no JavaScript engine bundled!

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
- Ahead-of-time compilation to a compact native app definition
- Compiler-directed retained-branch invalidation
- Native Vulkan rendering with `wgpu`
- Public Sans and Material Symbols
- Tabs and stack navigation
- Shared and persisted state
- Async resources and native actions
- Typed HTTPS data resources
- Cached reads, typed mutations and strict resource composition
- Computed values and compiler-checked route data
- Conditional and repeated UI
- Momentum scrolling
- Lightweight text input and keyboard
- Local and remote images, plus generated app icons
- Audio playback, recording and microphone analysis
- Foreground device location through LightOS permissions
- Screen-scoped NFC tag and NDEF reading
- Durable local notifications, UnifiedPush delivery and route-aware notification taps
- In-screen photo capture and QR/barcode scanning
- Multi-file apps and installed UI packages
- Android development, signing and device tooling through the `ink` CLI

## Modules

- `ink` — UI, layout, navigation, state and persistence
- `@ink/light-sdk` — LightOS preferences, hardware keys, dialler, ringtone and push
- `@ink/network` — Typed HTTPS JSON resources
- `@ink/notifications` — Local reminders, permission and durable tap events
- `@ink/audio` — Playback, recording and microphone analysis
- `@ink/location` — Foreground device location and permission
- `@ink/camera` — Permission, photo capture and code scanning
- `@ink/nfc` — NFC tag and NDEF reading
- `@ink/background` — Typed, persisted periodic JSON resources

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

## LightOS support

Enable LightOS integration with one import:

```tsx
import "@ink/light-sdk";
```

Native values are lazy, screen-scoped resources with typed loading, ready and error states:

```tsx
import { lightSdkVersion } from "@ink/light-sdk";
import { Button, Screen, Text, match } from "ink";

export default function Info() {
  const version = lightSdkVersion();

  return (
    <Screen title="Light SDK">
      {match(version, {
        loading: () => <Text>Connecting...</Text>,
        ready: (result) => <Text>{result.value}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      <Button onPress={() => version.reload()}>Refresh</Button>
    </Screen>
  );
}
```

`match` is exhaustive and narrows each branch to its status. It compiles to native conditionals and adds no runtime dependency.

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
- [x] Keyboard preferences
- [x] Permission status
- [x] Permission requests
- [x] Device key forwarding
- [x] Secure ringtone file hand-off
- [x] UnifiedPush registration and delivery
- [x] Background message notifications
- [x] Open dialler
- [x] Set ringtone

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

## NFC

`@ink/nfc` reads one NFC tag while its screen is active:

```tsx
import { nfcTag } from "@ink/nfc";

const tag = nfcTag({ timeoutMs: 30_000 });

<Button onPress={() => tag.reload()}>Read another tag</Button>
```

The result includes the tag serial number and normalised text, URI and binary NDEF records. A tag without NDEF data still succeeds with an empty record list. Apps that do not import the module carry neither its native adapter nor NFC manifest declarations.

## Camera

`@ink/camera` provides an Ink-owned camera preview for reviewed JPEG capture and ZXing code scanning:

```tsx
import { CameraPreview, photoCapture } from "@ink/camera";
import { Screen } from "ink";

export default function Photo() {
  const capture = photoCapture();

  return (
    <Screen title="Photo">
      <CameraPreview session={capture} />
    </Screen>
  );
}
```

Camera permission, photo capture and scanning are packaged independently, so applications pay only for the capabilities they declare. See [Camera](docs/camera.md) for session and lifecycle semantics.

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
- `ink logs --resources` shows native request transitions and timings.
- `ink info` explains retained rendering paths, module capabilities, native cost and app data size.
- `ink doctor` checks the development environment.

Use `ink -C <directory> <command>` to run a command for an app outside the current directory.

## Documentation

- [Architecture](docs/architecture.md)
- [LightOS capabilities](docs/light-sdk.md)
- [Data and effects](docs/data.md)
- [Notifications](docs/notifications.md)
- [Background resources](docs/background.md)
- [Audio](docs/audio.md)
- [Location](docs/location.md)
- [NFC](docs/nfc.md)
- [Camera](docs/camera.md)
- [Performance](docs/performance.md)
