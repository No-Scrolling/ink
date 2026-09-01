---
title: "Core Ink"
description: "Build interfaces with Ink's UI, navigation, state, and input APIs."
---

The `ink` package provides UI, navigation, state, and the shared types used by every optional Ink module. It is a compile-time package: your APK contains the native Ink runtime, not a JavaScript engine or a copy of this TypeScript package.

## App structure

Every app starts in `App.tsx`. A small app can define its only screen there. Larger apps can import screens from separate files and compose them with `Navigator`, `Route`, `Tabs`, and `Tab`.

```tsx
// App.tsx
import { Navigator, Route, Tab, Tabs } from "ink";
import Home from "./screens/Home";
import Settings from "./screens/Settings";
import Temperature from "./screens/Temperature";

export default function App() {
  return (
    <Navigator>
      <Route path="/">
        <Tabs>
          <Tab icon="home"><Home /></Tab>
          <Tab icon="settings"><Settings /></Tab>
        </Tabs>
      </Route>
      <Route path="/settings/temperature">
        <Temperature />
      </Route>
    </Navigator>
  );
}
```

Screen components take no props. Use route data, shared state, or persisted state when separate screens need to exchange values.

## Organise screens and packages

Put each tab or nested page in its own `.tsx` file, and keep `App.tsx` focused on navigation. A screen module exports one zero-argument component as its default export.

You can also publish screen modules as installed Ink packages. Add an `ink` export condition that points to the same TSX source as the type export:

```json
{
  "name": "@example/player-ui",
  "version": "1.0.0",
  "exports": {
    "./now-playing": {
      "types": "./NowPlaying.tsx",
      "ink": "./NowPlaying.tsx"
    }
  },
  "peerDependencies": {
    "ink": "*"
  }
}
```

Import the screen through its package name:

```tsx
import NowPlaying from "@example/player-ui/now-playing";
```

Ink resolves packages through `node_modules` and compiles their source with the same language checks as local screens. It does not execute package JavaScript.

## Screens and layout

`Screen` owns the standard Ink header, content insets, scrolling, and safe space above bottom navigation. Set `title` to show a header, or omit it for a headerless screen. `centered` centres the content in the available area.

When the content is taller than the screen, Ink adds a scrollbar automatically. Drag its thumb to move through the page, or tap the track to centre the thumb at that position.

`Stack` arranges children vertically by default. Set `axis="horizontal"` for a row.

```tsx
<Screen title="Now Playing">
  <Stack gap={16}>
    <Text>Song title</Text>
    <Stack axis="horizontal" gap={12} align="center" justify="space-between">
      <Button>Previous</Button>
      <Button>Next</Button>
    </Stack>
  </Stack>
</Screen>
```

| `Stack` prop | Values | Default |
| --- | --- | --- |
| `axis` | `"vertical"`, `"horizontal"` | `"vertical"` |
| `gap` | A non-negative logical size | `0` |
| `align` | `"start"`, `"center"`, `"end"`, `"stretch"` | `"stretch"` |
| `justify` | `"start"`, `"center"`, `"end"`, `"space-between"` | `"start"` |

`align` controls the cross axis. `justify` controls the direction in which the stack lays out its children.

## Text and controls

Ink uses Public Sans throughout the app. Emoji use the device's system emoji font, so apps can render the full emoji set supported by their Android version without bundling an emoji font or image set. Text is fully opaque and uses the framework's default size unless you pass `size`.

```tsx
<Text size={40} align="center">18°</Text>
<Button icon="refresh" onPress={() => weather.reload()}>Refresh</Button>
<Toggle
  label="Invert colours"
  value={inverted.value}
  onChange={() => inverted.set(!inverted.value)}
/>
```

`Text` wraps automatically at Unicode line-break opportunities. If one word is wider than the available space, Ink breaks it at a grapheme boundary rather than clipping it. Set `maxLines` to limit the result and truncate the final visible line with an ellipsis.

```tsx
<Text maxLines={2}>{description.value}</Text>
```

Set `align` to `"start"`, `"center"`, `"end"`, or `"justify"`. Justification expands only wrapped lines; the final line remains start-aligned.

Buttons, field values, headers, and text inputs stay on one line and use an ellipsis when their content is too wide.

Core controls are:

| Component | Purpose |
| --- | --- |
| `Text` | Renders text, numbers, and state values. |
| `Button` | Runs an action or opens a route. It can show one Material Symbol and an underline. |
| `Field` | Shows a label and its current value. Add `href` or `onPress` to make it actionable. |
| `Toggle` | Changes a `boolean` value. |
| `Icon` | Renders one referenced Material Symbol. |
| `TextInput` | Edits a string with the Ink keyboard. |
| `Image` | Renders a bundled image, an HTTPS image, or an opaque native image. |

Material Symbols are referenced by name. Ink includes only the symbols used by the app. General icons use the outlined style; bottom navigation uses filled symbols.

## Text input

`TextInput` is a controlled input. Store its value in state and update that state from `onChange`.

```tsx
const query = state("");

<TextInput
  placeholder="Search"
  value={query.value}
  onChange={(value) => query.set(value)}
  action="search"
/>
```

`action` controls the bottom-right keyboard key and accepts `"search"`, `"return"`, or `"done"`. The emoji keyboard uses the same system emoji as app text and follows the configured LightOS emoji list when LightOS integration is enabled. Apps without `TextInput` do not include the keyboard.

Add `autoFocus` to select an input and open the keyboard when its screen becomes active. It focuses once per screen visit, so dismissing the keyboard does not immediately reopen it.

## Images

`Image` requires a width and height in Ink logical units. `fit="cover"` fills the bounds and may crop the source; `fit="contain"` keeps the complete source visible. Use `bleed` when an image should extend through the normal horizontal content inset.

```tsx
<Image
  src="./assets/cover.png"
  width={349}
  height={349}
  fit="cover"
  bleed
/>
```

An HTTPS URL loads a remote image. `fallback` may point to a bundled image shown when the remote request fails. Native modules such as the camera return an opaque `ImageSource` that can be passed directly to `src`.

Bundled images may be PNG or JPEG files. Add `zoomable` to support pinch-to-zoom and one-finger panning while zoomed. Repeated double taps move through 2×, 3×, and 4×, then return to the original size. A full-bleed, zoomable image that is the only item on a screen fills the area below the header. Ink keeps these interactions native and updates only the image transform while it moves.

```tsx
<Image
  src="./assets/photo.jpg"
  width={349}
  height={349}
  fit="contain"
  bleed
  zoomable
/>
```

## State

Ink provides three kinds of writable state:

| Function | Lifetime |
| --- | --- |
| `state(initial)` | Belongs to one screen instance. |
| `sharedState(key, initial)` | Shared by declarations with the same key while the app is running. |
| `persistedState(key, initial)` | Shared by key and restored after the app restarts. |

```tsx
const count = state(0);
const selectedTab = sharedState("player.tab", "queue");
const temperatureUnit = persistedState("settings.temperature", "Celsius");

<Field label="Temperature" href="/settings/temperature">
  {temperatureUnit.value}
</Field>
```

Scalar state supports `boolean`, `number`, `string`, and `null`. List state also provides `append`, `remove`, `replace`, and `clear`.

```tsx
const places = persistedState("places", ["London"]);

<Button onPress={() => places.append("Paris")}>Add Paris</Button>
```

`computed` derives a scalar value from state or resource values. The compiler turns the expression into Ink's native value graph.

```tsx
const count = state(2);
const doubled = computed(() => count.value * 2);

<Text>{doubled.value}</Text>
```

## Conditional and repeated UI

Use normal JSX conditions and `.map()` over an Ink list value. Ink updates the affected native UI when the source value changes.

```tsx
{places.value.length === 0 ? (
  <Text>No saved places</Text>
) : (
  <Stack gap={16}>
    {places.value.map((place) => <Text>{place}</Text>)}
  </Stack>
)}
```

Long, fixed-height vertical lists are virtualised automatically. Ink lays out only the visible rows and a small overscan area.

## Navigation

Wrap multi-page apps in `Navigator` and give every destination a compile-time `Route` path. A `Button` can navigate instead of running an action.

Nested routes include a back button automatically. You can also swipe right from the left edge to return. Vertical gestures that begin at the edge continue to scroll the page normally.

```tsx
<Button href="/settings/temperature">Temperature</Button>
```

Call `back()` to return after an action:

```tsx
<Button onPress={() => {
  temperatureUnit.set("Celsius");
  back();
}}>
  Celsius
</Button>
```

### Route data

Pass scalar route data with an object `href`. The destination declares its contract with `routeParams<T>()`. Ink checks the route and its data at build time.

```tsx
// Source screen
<Button href={{ path: "/forecast", params: { city: "London" } }}>
  London
</Button>

// Destination screen
const params = routeParams<{ city: string }>();

<Text>{params.city}</Text>
```

### Tabs

Place `Tabs` inside a route and add one `Tab` for each root screen. The `icon` is a Material Symbol name. Ink owns tab selection and renders its standard bottom navigation.

## Async values

Native and network reads expose tagged states such as `loading`, `ready`, and `error`. Use `match` to render every state. TypeScript narrows the value inside each branch and reports a missing branch.

```tsx
{match(location, {
  loading: () => <Text>Finding location</Text>,
  ready: (result) => <Text>{result.value.latitude}</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

Use `all` when a screen needs several resources before it can render. Read [Data and effects](data.md) for network reads, mutations, caching, and resource composition.

## Errors and permissions

Ink module errors provide a stable `kind`, a plain `message`, and a `retryable` flag. Permission resources add a `request()` action and return `"granted"`, `"denied"`, `"blocked"`, or `"unknown"` when ready. Constructing a permission resource never opens a prompt; call `request()` from a user action.

## Supported TypeScript

Ink accepts a focused TypeScript and TSX syntax that can be checked and compiled ahead of time. It does not execute arbitrary JavaScript or support general JavaScript packages. Local screens and installed Ink UI packages use the same supported syntax.

Run `ink check` for source diagnostics, and run `ink info` to see which state, resources, permissions, and native modules the app includes. Read [How Ink works](architecture.md) for the build and runtime model.
