---
title: "Build with Ink"
description: "Small TypeScript apps with a native Light Phone interface."
tag: "Design specification"
---

Build Light Phone III apps with React and TypeScript. Ink provides native components, navigation, rendering and device APIs.

> **Design specification.** These docs describe the intended Ink APIs and development experience. Some interfaces and commands are not yet implemented.

## Start an app

```sh
bun create ink weather
cd weather
bun install
bun run dev
```

The generated project contains `App.tsx`, `ink.toml`, a TypeScript configuration and scripts for the Ink CLI. `ink dev` builds and runs on a connected LP3 or Android emulator. Development changes to JavaScript reload the app; adding a native package requires rebuilding its development APK. A reload disposes the previous JavaScript runtime and its subscriptions.

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
| `Button` | A text action, optional icon, disabled state or `href`. |
| `Field` | A label and value, optionally actionable. |
| `Toggle` | A labelled boolean input. |
| `TextInput` | Controlled input with Ink's native keyboard. |
| `Image` | Bundled, remote or managed-file imagery with native zooming. |
| `List` | Keyed, virtualised rows for collections. |
| `Screen`, `Stack` | Page structure and layout. |

Keep button labels and important values readable without relying on truncation. Give icon-only controls an `accessibilityLabel`. Native controls expose roles, labels, values and actions to Android accessibility. Decorative images can be marked as such; meaningful images need descriptions.

## Input and images

```tsx
import { useState } from "react";
import { Image, Screen, TextInput } from "ink";

export function Search() {
  const [query, setQuery] = useState("");
  return <Screen title="Search">
    <TextInput value={query} onChange={setQuery} placeholder="Search places" action="search" />
    <Image src="./assets/cover.jpg" width={280} height={280} fit="contain" zoomable />
  </Screen>;
}
```

Input actions are `search`, `return` and `done`; `autoFocus` focuses once per screen visit. The keyboard and cursor interaction stay native. LightOS preferences inform haptics and keyboard behaviour where the host exposes them.

Remote images use HTTPS. Images require dimensions or a bounded parent, can use `fit="contain"` or `"cover"`, and can supply a bundled fallback. Decoding, downsampling, texture caching, pinch zoom and panning stay native. Large media bytes need not pass through JavaScript.

## Collections

```tsx
<List
  items={places}
  keyExtractor={place => place.id}
  estimatedItemHeight={64}
  renderItem={place => <Button onPress={() => openPlace(place.id)}>{place.name}</Button>}
  empty={<Text>No saved places</Text>}
/>
```

Use stable domain IDs, not array positions. `List` requests rows around the visible region and reuses retained native content while scrolling. Variable-height rows are supported; an accurate estimate helps preserve position as rows are measured. `.map()` is fine for a short group but does not automatically virtualise a large collection.

Pagination belongs to the data module. Keep loaded data bounded; row virtualisation does not reduce an array already loaded into JavaScript.

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

`navigate({ path: "/forecast", params: { placeId } })` and an equivalent `href` push a destination. `back()` returns; `replace()` replaces the current entry. Headers, edge-back gestures and hardware Back use the same navigation stack. Pass IDs and small JSON values, not an entire message history or a live player.

`useRouteParams<T>()` gives an internal route its declared TypeScript shape. A generic alone cannot validate a deep link: pass a `decode(unknown)` function when data can arrive externally. Invalid links open a controlled not-found/error screen. Route paths are statically registered so external entry points can be packaged; parameter values are dynamic.

A pushed screen retains its local state while covered. Tabs retain independent state and scroll positions. Visibility-scoped work pauses when a screen is covered or its tab is inactive. Popping a screen disposes it. None of that makes local state durable across process death.

## Build and inspect

```sh
ink check
ink info
ink build
ink install
ink logs
```

`ink check` checks TypeScript, route declarations, assets, host API compatibility and native package contracts. It does not prove every dynamic execution path. `ink info` explains the JavaScript bundle, linked native packages, permissions and size contributions. `ink build` produces a release APK using the signing configuration in `ink.toml`.
