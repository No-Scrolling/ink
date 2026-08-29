# Ink

Ink is an experimental Android UI framework for small Light Phone III apps. Apps are written in a deliberately restricted TypeScript/TSX dialect and compiled ahead of time into Rust. There is no JavaScript runtime in the APK.

The first vertical slice is a native counter rendered by `wgpu`, with Public Sans Regular rasterised into a small glyph atlas by Ink's compact text path.

![Ink counter running on the LP3 emulator](docs/images/counter.png)

## Counter

```tsx
import { Button, Column, Screen, Text, state } from "ink";

export default function Counter() {
  const count = state(0);

  return (
    <Screen>
      <Column gap={16}>
        <Text size={32}>Count: {count.value}</Text>
        <Button onPress={() => count.set(count.value + 1)}>Increase</Button>
      </Column>
    </Screen>
  );
}
```

## Commands

- `ink check` validates the application without producing build artefacts.
- `ink build` produces an optimised, release-signed APK in `dist/`.
- `ink build --debug` produces an automatically development-signed APK.
- `ink dev` builds, installs and launches the application, then watches for changes.
- `ink devices` lists connected devices and the remembered default.
- `ink logs` streams package-scoped application and crash logs.
- `ink info` shows resolved application, signing, device and build information.
- `ink doctor` checks the Rust and Android development environment.

Ink discovers `ink.toml` from the current directory. Use `ink -C <directory> <command>` to work with an application elsewhere, such as `ink -C examples/counter dev` from this repository's root.

`ink dev --logs` keeps Logcat alongside the development watcher. Use `--device <serial-or-name>` to select a device; Ink remembers its serial for later commands.

## Application metadata

Application identity and Android versioning live in `ink.toml`:

```toml
name = "Counter"
package = "com.vandam.counter"
version = "0.1.0"
version_code = 1
```

Debug builds use Android's local development key automatically. A release build requires non-secret key metadata in `ink.toml`:

```toml
[signing]
keystore = "release.jks"
key_alias = "upload"
```

Supply passwords through `INK_KEYSTORE_PASSWORD` and `INK_KEY_PASSWORD`; never store them in the repository. Ink asks Gradle to sign the APK and verifies the resulting signature before copying it to `dist/`.

The current signed, optimised arm64 APK is approximately 2.4 MB. The development APK is intentionally unoptimised and much larger.

Ink currently targets Android API 34 or newer. The saved LP3 emulator and physical LP3 builds are arm64-only.

## Status

Ink is a narrow prototype. The compiler accepts only the language exercised by the counter. Light SDK and Light Keyboard integrations will be adapters once the core source and rendering interfaces have settled.
