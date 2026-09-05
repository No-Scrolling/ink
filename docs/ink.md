---
title: "Build with Ink"
description: "Small TypeScript apps with a native Light Phone interface."
tag: "In development"
---

> **In development.** React apps, native components, navigation, fixed-height lists and native subtree updates are implemented.

Build Light Phone III apps with React and TypeScript. Ink provides native components, navigation, rendering and device APIs.


## Run the template

From the Ink repository with the Android toolchain installed:

```sh
bun install
cargo run -p ink-cli -- -C examples/light-template check
cargo run -p ink-cli -- -C examples/light-template dev --device emulator-5554 --once
```

Use `ink devices` to find the serial of a connected LP3 or emulator. The template contains `App.tsx`, `ink.toml` and TypeScript configuration. `ink dev` builds, installs and launches the APK; omit `--once` to watch for changes and rebuild. A published `bun create ink` starter and JavaScript-only reload are planned.

```tsx
import { useState } from "react";
import { Button, Screen, Text } from "ink";

export default function App() {
  const [count, setCount] = useState(0);
  return (
    <Screen title="Counter" centered>
      <Text size={48}>{count}</Text>
      <Button onPress={() => setCount(value => value + 1)}>Increase</Button>
    </Screen>
  );
}
```

## An Ink screen

```tsx
import { useState } from "react";
import { Button, Field, Screen, Stack, Text, Toggle } from "ink";

export default function Settings() {
  const [offlineOnly, setOfflineOnly] = useState(false);
  return (
    <Screen title="Settings">
      <Stack gap={24}>
        <Field label="Download quality">Standard</Field>
        <Toggle label="Show downloaded items" value={offlineOnly} onChange={setOfflineOnly} />
        <Text>Downloaded items are available without a connection.</Text>
        <Button href="/downloads">Manage downloads</Button>
      </Stack>
    </Screen>
  );
}
```

`Screen` owns the header, back affordance, content insets and safe space above tabs. It scrolls ordinary content when needed. `Stack` arranges children vertically; `axis="horizontal"` creates a row. Use `gap`, `align` and `justify` rather than repeated spacer elements.

Ink uses Public Sans, strong contrast, clear text and a small set of familiar controls. Sizes are Ink logical units. Colours follow the app's light or dark appearance. General icons are Material Symbols; declare the icons your app can select dynamically so they can be bundled.

| Component | Use |
| --- | --- |
| `Text` | Wrapping text; `size`, `align` and `maxLines` control presentation. |
| `Button` | A text action, optional icon, selected or disabled state, or `href`. Only selected buttons are underlined. |
| `Field` | A single-line label and wrapping value, optionally actionable. The field grows with its value and keeps the whole area tappable. |
| `Toggle` | A labelled boolean input. |
| `TextInput` | Controlled input with Ink's native keyboard. |
| `Image` | Bundled, remote or managed-file imagery with native zooming. |
| `List` | Virtualised fixed-height rows using the screen’s native scrolling. |
| `Screen`, `Stack` | Page structure and layout. |

Keep button labels and important values readable without relying on truncation.

`useAction(operation)` tracks an asynchronous command as `idle`, `pending`, `success` (with `data`) or `error` (with `error`). Call `action.run(...args)` from a control and disable it while pending. Repeated calls while pending are ignored; failures become state rather than unhandled promise rejections. Unmounting the component or hiding it through navigation discards its eventual UI result, but does not cancel the underlying operation.

## Common screen patterns

Compose these inside a `Screen`; they use the same native text, controls and spacing as other Ink content:

| Component | Interface |
| --- | --- |
| `SettingsChoices` | `options` with stable `value` and `label`, current `value`, `onChange`, and optional `disabled`. The selected choice is underlined. |
| `LoadingState` | Optional `label`, defaulting to “Loading…”. |
| `EmptyState` | `title`, optional `description`, and optional `action` containing a label and press handler. |
| `ErrorState` | `message`, `onRetry`, optional `retryLabel` and `disabled`. |
| `Confirmation` | A complete screen: `title`, small text children, `confirmLabel`, `onConfirm`, and optional pending state and label. One uppercase action is anchored at the bottom; Back cancels. |

These compositions do not fetch data, persist choices or navigate automatically. Supply those behaviours through callbacks, using `useAction` to track asynchronous work when useful. Keep a failed action's error visible and allow retry. Use a consequence such as “Clear example list” for confirmation rather than “Yes”.

The template demonstrates each pattern: Search submits from the keyboard to a separate results page, Screen States runs an explicitly local loading/error/retry demonstration, and Confirmation opens the single-action screen directly and returns when confirmed.

`Confirmation` owns its screen; do not wrap it in another `Screen`. Its message can scroll when necessary while the action stays at the bottom. Pending state disables the action and displays `pendingLabel` (default “Working…”).

```tsx
<Confirmation title="Delete List" confirmLabel="Delete" onConfirm={deleteList}>
  Are you sure you want to delete this list and all its items?
</Confirmation>
```

## Input and images


```tsx
import { useState } from "react";
import { navigate, Screen, TextInput } from "ink";

export function Search() {
  const [query, setQuery] = useState("");
  return <Screen title="Search">
    <TextInput value={query} onChange={setQuery} placeholder="Search places" action="search"
      onSubmit={value => { if (value.trim()) navigate({ path: "/search-results", params: { query: value } }); }} />
  </Screen>;
}
```

Submitting to an `onSubmit` handler preserves focus; navigating away closes the keyboard with the page change. Without a handler, submission dismisses the keyboard. Input actions are `search`, `return` and `done`; `autoFocus` focuses once per screen visit. The keyboard and cursor interaction stay native. The keyboard reserves space below the screen, and viewport changes reveal the focused input. LightOS preferences inform haptics and keyboard behaviour where the host exposes them. Use a dedicated input screen with its action in the keyboard, and show results on a separate page. Inputs should stay outside scrolling content; a scrollbar belongs only to any separate content region below an input. `Field` displays a text value and cannot contain an input.

Centred images in a full-width vertical stack stay centred on the page when it scrolls; text and controls retain space for the scrollbar.

Remote images use HTTPS. Images require dimensions or a bounded parent, can use `fit="contain"` or `"cover"`, and currently have no fallback prop. Decoding, downsampling, texture caching, pinch zoom and panning stay native. Large media bytes need not pass through JavaScript. For a zoomable image, double-tap cycles through 2×, 3× and 4× magnification, then resets to the fitted image; drag to pan while zoomed. Pinch interaction still needs device verification.

## Collections

Use `.map()` with stable keys for small collections. For large collections, `List` mounts the visible rows with a viewport of extra rows on either side:

```tsx
<List
  items={songs}
  itemHeight={64}
  keyExtractor={song => song.id}
  renderItem={song => <Button onPress={() => play(song)}>{song.title}</Button>}
/>
```

`itemHeight` is the full height of every row in Ink layout units. Keep each row’s content within that height; variable-height rows are not supported. `List` uses its containing screen’s scroll position and preserves the full collection’s scroll extent. Keys must be stable and unique. Rows outside the window unmount, so keep durable row state in the parent or a store. The initial window contains up to 32 rows until the native viewport arrives.

## Navigation

```tsx
import { Navigator, Route, Tab, Tabs } from "ink";
import Home from "./screens/Home";
import Settings from "./screens/Settings";
import Forecast from "./screens/Forecast";

export default function App() {
  return (
    <Navigator>
      <Route path="/">
        <Tabs>
          <Tab id="home" icon="home"><Home /></Tab>
          <Tab id="settings" icon="settings"><Settings /></Tab>
        </Tabs>
      </Route>
      <Route path="/forecast"><Forecast /></Route>
    </Navigator>
  );
}
```

`navigate({ path: "/forecast", params: { placeId } })` and an equivalent `href` push a destination. `back()` returns; `replace()` replaces the current entry. Headers, edge-back gestures and hardware Back use the same navigation stack. Those native back affordances dismiss a focused keyboard first; a subsequent back returns to the previous screen. Pass IDs and small JSON values, not an entire message history or a live player.

`useRouteParams<T>()` gives an internal route its declared TypeScript shape. A generic alone cannot validate a deep link: pass a `decode(unknown)` function when data can arrive externally. An unknown notification route opens “Page unavailable” with a back action, preserving the previous screen. Registered route components are responsible for handling invalid parameter values. Parameter values are dynamic.

A pushed screen retains its local state while covered. Tabs retain independent state and scroll positions. Visibility-scoped work pauses when a screen is covered or its tab is inactive. Popping a screen disposes it. None of that makes local state durable across process death.

## Build and inspect

```sh
ink check
ink info
ink build
ink logs
```

`ink check` validates the project and type-checks JavaScript apps. It does not establish compatibility for every npm dependency or dynamic path. `ink info` shows resolved project/build information; a detailed bundle-size report is planned. `ink build` produces a release APK using the signing configuration in `ink.toml`; use `ink dev --once` to install and launch a development build.

Configure your release key in `ink.toml`, with the keystore path relative to that file:

```toml
[signing]
keystore = "release.keystore"
key_alias = "ink"
```

Set `INK_KEYSTORE_PASSWORD` in your shell or CI environment. Set `INK_KEY_PASSWORD` if the key uses a different password; otherwise Ink uses the keystore password. Keep the keystore and passwords out of version control, and retain the same signing key for app updates. A release signed with a different key cannot replace an installed development APK with the same application ID.

## App appearance

Ink starts with a dark appearance. `setColourScheme("light")` or `setColourScheme("dark")` changes the native palette without remounting screens. `useColourScheme()` subscribes to the current choice.

```tsx
import { setColourScheme, useColourScheme, Toggle } from "ink";

function AppearanceChoice() {
  const scheme = useColourScheme();
  return <Toggle label="Invert Colours" value={scheme === "light"}
    onChange={() => setColourScheme(scheme === "dark" ? "light" : "dark")} />;
}
```

The palette covers backgrounds, text, icons, controls, navigation and scrollbars. Photos, barcode pixels and camera previews keep their original colours. Ink’s built-in keyboard also follows the chosen palette.

The choice lasts for the JavaScript runtime. Persist it with `@ink/store` if required; the reference template demonstrates loading and applying a saved choice. LightOS does not currently expose its global inversion preference through its SDK, so there is no automatic system mode.
