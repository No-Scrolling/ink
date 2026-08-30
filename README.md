# Ink

Ink is an experimental Android UI framework for small Light Phone III apps. Apps are written in a deliberately restricted TypeScript/TSX dialect and compiled ahead of time into Rust. There is no JavaScript runtime in the APK.

The first vertical slice is a native component set rendered by `wgpu`, with Public Sans Regular rasterised into a small glyph atlas by Ink's compact text path.

![Ink counter running on the LP3 emulator](docs/images/counter.png)

## Counter

```tsx
import { Button, Screen, Stack, Text, state } from "ink";

export default function Counter() {
  const count = state(0);

  return (
    <Screen title="Counter" centered>
      <Stack gap={16} align="center">
        <Text size={40}>Count: {count.value}</Text>
        <Button onPress={() => count.set(count.value + 1)}>Increase</Button>
      </Stack>
    </Screen>
  );
}
```

## Screen modules

`App.tsx` is the composition root, so tabs, routes and their ordering remain visible in one place. Page content and page-local state can live in relative `.tsx` screen modules:

```tsx
import { Navigator, Route } from "ink";
import Settings from "./screens/Settings";

export default function App() {
  return (
    <Navigator>
      <Route path="/">
        <Settings />
      </Route>
    </Navigator>
  );
}
```

```tsx
// screens/Settings.tsx
import { Screen, Toggle, state } from "ink";

export default function Settings() {
  const enabled = state(false);

  return (
    <Screen title="Settings">
      <Toggle
        label="Enabled"
        value={enabled.value}
        onChange={() => enabled.set(!enabled.value)}
      />
    </Screen>
  );
}
```

Screen modules use one default export, take no arguments and return `Screen`. They may appear directly inside `Route` or `Tab`. Ink follows extensionless relative imports to `.tsx` files inside the application, expands every screen at compile time and keeps its state local to that screen instance. There is no runtime module loader or JavaScript cost.

## Shared and persisted state

State lifetime is selected where the value is declared:

```tsx
const expanded = state(false);
const selectedItem = sharedState("selected-item", "");
const invertColours = persistedState("settings.invert-colours", false);
```

`state` belongs to one screen instance. `sharedState` is app-wide until the process exits. `persistedState` is app-wide and survives relaunches and upgrades in Android's app-private storage. Shared and persisted values use a stable string key; declaring the same key in separate screens links both declarations to one value. Repeated declarations must use the same lifetime, type and initial value or `ink check` reports the conflict.

Hydration finishes before Ink attaches its first surface, and saving is automatic. Apps do not need providers, effects, asynchronous loading or storage packages. Persisted state is intended for settings and small app metadata rather than images, media, caches or secrets. Changing a key or its value shape resets that key to its compiled initial value.

Selection is composed from ordinary buttons. Add `back()` after the state update to return immediately, or omit it to keep the selector open:

```tsx
<Button
  underline={temperatureUnit.value === "Celsius"}
  onPress={() => {
    temperatureUnit.set("Celsius");
    back();
  }}
>
  Celsius
</Button>
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

Ink includes its lightweight keyboard automatically when an app uses `TextInput`. Its Android module and resources are omitted from apps without text input.

## Dynamic UI

Ink supports conditional elements and rendering homogeneous state lists with ordinary TSX. Both are compiled into native Ink nodes; they do not add a JavaScript runtime.

```tsx
const showApps = state(true);
const apps = state<{ name: string; description: string }[]>([]);

<Stack gap={16}>
  {showApps.value && <Text>Favourite apps</Text>}
  {apps.value.length === 0 ? (
    <Text>No apps yet</Text>
  ) : (
    <Stack gap={8}>
      {apps.value.map((app) => (
        <Stack>
          <Text>{app.name}: {app.description}</Text>
          <Button onPress={() => apps.remove(app)}>Remove</Button>
        </Stack>
      ))}
    </Stack>
  )}
</Stack>
```

Empty lists use an explicit inline array type; non-empty literal lists infer their shape. Every item must have the same shape. Lists support `set`, `append`, `remove`, `replace` and `clear`, while `value.length` works in text and empty-state conditions. Removal and replacement use the current mapped item, with row identity handled by Ink.

## Application metadata

Application identity and Android versioning live in `ink.toml`:

```toml
name = "Counter"
package = "com.vandam.counter"
version = "0.1.0"
version_code = 1
```

Ink uses `App.tsx` as the application entry point and follows its relative screen imports. Generated Rust, launcher icons and other build inputs stay in the application's ignored `.ink/` directory; their paths are framework implementation details.

Debug builds use Android's local development key automatically. A release build requires non-secret key metadata in `ink.toml`:

```toml
[signing]
keystore = "release.jks"
key_alias = "upload"
```

Supply passwords through `INK_KEYSTORE_PASSWORD` and `INK_KEY_PASSWORD`; never store them in the repository. Ink asks Gradle to sign the APK and verifies the resulting signature before copying it to `dist/`.

The current signed, optimised arm64 base APK is approximately 2.9 MB. The development APK is intentionally unoptimised and much larger.

Ink currently targets Android API 34 or newer. The saved LP3 emulator and physical LP3 builds are arm64-only.

Style values are authored directly in Ink's LP3 logical units. At the LP3's 1080-pixel width they match the template's established 2.55-pixel scale, without an application-side scaling helper.

## Components

- `Screen` owns the app header, content insets, vertical rhythm, overflow scrolling and scroll indicator.
- `Stack` arranges children vertically or horizontally with optional gap, alignment and distribution.
- `Text` uses Public Sans and always renders at the full foreground colour. It supports an optional size and alignment, but no opacity or muted-text styling.
- `TextInput` binds to string state and uses Ink's lightweight keyboard renderer for touch editing. The keyboard shares Ink's embedded Public Sans bytes rather than bundling another font. It accepts `action="search"`, `"return"` or `"done"`; the bottom close control and Android back dismiss the keyboard.
- `Button` is a text-first action with an optional Material Symbol and underline.
- `Icon` accepts a Material Symbol name. Ink uses the outlined family at weight 300 and embeds only the masks referenced by the app.
- `Image` embeds a local PNG at compile time with `cover` or `contain` fitting.
- `Toggle` provides the established LP3 line-and-circle setting control.
- `Tabs` and `Tab` own the fixed bottom navigation bar and screen switching. Tab icons use the filled Material Symbols variant at weight 400.
- `Navigator` and `Route` declare a compile-time checked screen graph. Buttons navigate with `href`; nested screens receive an automatic back control and Android back uses the same history.

```tsx
const query = state("");

<TextInput
  placeholder="Search..."
  value={query.value}
  onChange={(value) => query.set(value)}
  action="search"
/>
```

```tsx
<Navigator>
  <Route path="/">
    <Screen title="Settings">
      <Button href="/settings/interface">Interface</Button>
    </Screen>
  </Route>
  <Route path="/settings/interface">
    <Screen title="Interface">
      <Text>Display settings</Text>
    </Screen>
  </Route>
</Navigator>
```

`examples/counter` is the smallest interactive example: one screen, one state value and one action. `examples/light-template` mirrors the three top-level template pages for visual comparisons with the React Native and Light SDK components.

## Status

Ink remains a deliberately narrow prototype. Route parameters and Light SDK resources are not implemented yet. Those integrations will sit behind framework-owned adapters rather than expanding every component's surface.
