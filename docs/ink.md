---
title: "Build with Ink"
description: "Small TypeScript apps with a native Light Phone interface."
---

Build Light Phone III apps with React and TypeScript. Ink provides native components, navigation, rendering and device APIs.


## Planned external actions

Opening external links and sharing text are planned framework operations; no public API is available yet. They should handle the external activity round trip and distinguish cancellation from failure. File sharing belongs with the planned [Files and media](files.md) capability. Appearance and screen lifecycle continue to use Ink's existing APIs.

## Run the template

From the Ink repository with the Android toolchain installed:

```sh
bun install
cargo run -p ink-cli -- -C examples/light-template check
cargo run -p ink-cli -- -C examples/light-template dev --device emulator-5554 --once
```

Use `ink devices` to find the serial of a connected LP3 or emulator. The template contains `App.tsx`, `ink.toml` and TypeScript configuration. `ink dev` installs the development host and launches the app; omit `--once` to watch for changes. Compatible edits use JavaScript reload or refresh, while native changes rebuild the APK. See [the development loop](development.md) and [standalone setup](standalone.md).

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

Ink uses Public Sans, strong contrast, clear text and a small set of familiar controls. Sizes are Ink logical units. Colours follow the app's light or dark appearance. General icons are imported Material Symbol collections. `Icon.name`, `Button.icon` and `Tab.icon` accept imported icon references; `Icon.size` scales the raster mask, and `tone="muted"` uses the app’s muted colour. Local image/audio files must also be imported; see [assets and native requirements](build-contracts.md) for formats and migration.

| Component | Use |
| --- | --- |
| `Text` | Wrapping text; `size`, `align` and `maxLines` control presentation. |
| `Button` | A text action, optional icon, selected or disabled state, or `href`. Only selected buttons are underlined. |
| `Field` | A single-line label and wrapping value, optionally actionable. The field grows with its value and keeps the whole area tappable. |
| `Toggle` | A labelled boolean input. |
| `TextInput` | Controlled input with Ink's native keyboard. |
| `Image` | Imported image assets, HTTPS images and camera-managed images, with optional native zooming. |
| `List` | Virtualised, automatically content-measured rows using the screen’s native scrolling. |
| `Screen`, `Stack` | Page structure and layout. |
| `Row` | Tappable title/subtitle with optional artwork. |
| `PlayingScreen` | Player layout; the app supplies playback state and callbacks. |
| `ConversationScreen`, `Message` | Conversation presentation and interaction, or a standalone message. |

Keep button labels and important values readable without relying on truncation. `Text`, `Button`, `Field` values and `Confirmation` messages accept text content, including components that produce text. Nested `Text` is flattened: the outer text style applies, so nested size/alignment props do not create styled spans. Layout controls and inputs must be siblings rather than text children.

`useAction(operation)` tracks an asynchronous command as `idle`, `pending`, `success` (with `data`) or `error` (with `error`). Call `action.run(...args)` from a control and disable it while pending. Repeated calls while pending are ignored; failures become state rather than unhandled promise rejections. Unmounting the component or hiding it through navigation discards its eventual UI result, but does not cancel the underlying operation.

## Common screen patterns

Compose the state and settings patterns inside a `Screen`; `Confirmation` owns its screen. They use the same native text, controls and spacing as other Ink content:

| Component | Interface |
| --- | --- |
| `SettingsChoices` | `options` with stable `value` and `label`, current `value`, `onChange`, and optional `disabled`. The selected choice is underlined. |
| `LoadingState` | Optional `label`, defaulting to “Loading…”, and `align` (`start`, `center` or `end`). |
| `EmptyState` | `title`, optional `description`, and optional `action` containing a label and press handler. |
| `ErrorState` | `message`, `onRetry`, optional `retryLabel` and `disabled`. |
| `Confirmation` | A complete screen: `title`, small text children, `confirmLabel`, `onConfirm`, optional `centered` message layout, and optional pending state and label. One uppercase action is anchored at the bottom; Back cancels. |

These compositions do not fetch data, persist choices or navigate automatically. Supply those behaviours through callbacks, using `useAction` to track asynchronous work when useful. Keep a failed action's error visible and allow retry. Use a consequence such as “Clear example list” for confirmation rather than “Yes”.

The template demonstrates each pattern: Search submits from the keyboard to a separate results page, Screen States offers separate local Loading and Error examples: centred loading text leads to content or a centred error with a bottom retry action, and Confirmation opens the single-action screen directly and returns when confirmed.

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

The `search` and `done` actions submit to `onSubmit`; navigating away closes the keyboard with the page change. Without a handler, submission dismisses the keyboard. `action="return"` enables multiline input: Return inserts a newline and the input grows to three visible lines. Drag vertically inside it to scroll longer drafts; typing reveals the cursor again. It does not call `onSubmit`. ConversationScreen uses Return and its separate send button. `autoFocus` focuses once per screen visit. The keyboard and cursor interaction stay native. The keyboard reserves space below the screen, and viewport changes reveal the focused input. LightOS preferences inform haptics and keyboard behaviour where the host exposes them. Use a dedicated search screen with its action in the keyboard, and show results on a separate page. `Field` displays a text value and cannot contain an input.

Update the controlled `value` synchronously in `onChange`, as above; debounce network requests or other effects instead. Native event counters protect against older commits while JavaScript is busy. They do not infer which later asynchronous value updates an application intended to keep.

Centred images in a full-width vertical stack stay centred on the page when it scrolls; text and controls retain space for the scrollbar.

Placeholders use the theme's muted grey. The keyboard has no bottom dismiss row; it ends with Ink's standard 20-unit bottom inset. Back dismisses it. Multiline edits retain the input's scroll position while the cursor remains visible, and only adjust it when necessary to reveal the cursor.

Remote images use HTTPS. Images require explicit positive `width` and `height`, can use `fit="contain"` or `"cover"`, and currently have no fallback prop. `bleed` extends an image to the viewport width while preserving the declared aspect ratio. `zoomable` enables gestures; it defaults to false. Arbitrary `file://` images are not accepted by `Image`. Decoding, downsampling, texture caching, pinch zoom and panning stay native. Large media bytes need not pass through JavaScript. For a zoomable image, double-tap cycles through 2×, 3× and 4× magnification, then resets to the fitted image; drag to pan while zoomed. Pinch interaction still needs device verification.

## Collections

Use `.map()` with stable keys for small collections. For large collections, `List` mounts the visible rows with a viewport of extra rows on either side. Row heights are measured automatically; `gap` adds spacing and `followEnd` follows additions only near the end. Stable keys preserve the visible scroll anchor. See [list behaviour and limits](lists.md).

For example:

```tsx
<List
  items={songs}
  keyExtractor={song => song.id}
  renderItem={song => <Button onPress={() => play(song)}>{song.title}</Button>}
/>
```

`List` uses its containing screen’s scroll position and preserves the full collection’s scroll extent. Keys must be stable and unique. Rows outside the window unmount, so keep durable row state in the parent or a store. The initial window contains up to 32 rows until the native viewport arrives.

## Navigation

```tsx
import { Navigator, Route, Tab, Tabs } from "ink";
import Home from "./screens/Home";
import Settings from "./screens/Settings";
import Forecast from "./screens/Forecast";
import icons from "./navigation.ink-icons";

export default function App() {
  return (
    <Navigator>
      <Route path="/">
        <Tabs>
          <Tab id="home" icon={icons.home}><Home /></Tab>
          <Tab id="settings" icon={icons.settings}><Settings /></Tab>
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

## Rows and title-bar actions

Use `Screen`'s optional `header` for content that stays below the navigation title while the body scrolls, such as actions above a list. Supply normal components; Ink applies the usual content spacing.

`Row` provides a whole-row press target with `title`, optional `subtitle` and optional `image` source. Text wraps, and image rows reserve a 50-unit square before loading. Supply `onPress` or `href` for interaction, as with Button. A row without either is display-only.

```tsx
<Row image={artwork} title="Album title" subtitle="Artist name" href="/album" />
<Screen title="Albums" rightAction={{ icon: icons.more_horiz, onPress: openActions }}>
  {/* content */}
</Screen>
```

`rightAction` places one icon/action in the navigation title bar. It is separate from `header`, which pins composed content below the title. The Examples tab demonstrates the more-horizontal icon opening Action Page.

The Code generation page links to all 13 supported formats, each displayed inside a centred Screen. QR Code encodes `Hello World!` at size 240 with a two-module white border. `size` sets the display width; height follows the generated image. QR stays square, PDF417 uses its encoded proportions, and linear codes use a compact bar height with a white border. Other formats use text or valid numeric samples as appropriate. Generation and display are handled by `Barcode` from `@ink/barcode/generate`.

## Playing screen

`PlayingScreen` groups optional artwork, title/artists, progress and transport controls at the top of the content area below the header. Its `actions` stay at the bottom and are distributed evenly. Omitting `image` centres the main group vertically between the header and bottom actions; a loading image reserves its space.

Supply `playing`, `onPlayPause`, `position` and `duration` (milliseconds), and optional `onSeek` for tap-to-seek. `title` has optional `onTitlePress`; each entry in `artists` has a `name` and optional `onPress`. Required `previous` and `next` actions accept `onPress`, optional `onLongPress`, optional `disabled` and optional `seconds: 5 | 10 | 30` to show seek icons instead of track controls. Each bottom action accepts `icon`, `onPress`, optional `selected` and `disabled`.

The screen owns presentation; the app owns playback and queue behaviour. The template's Image and No Image examples simulate progress without playing audio. Image uses Wallsocket artwork and track controls; No Image demonstrates backward 10-second and forward 30-second controls.

A long press invokes `onLongPress` once and suppresses the normal tap on release. Moving away cancels it. The template seeks backward/forward by 15 seconds on a hold, matching Reverb; the callback determines the amount. Bottom actions can change their `icon` independently of `selected`, which adds an underline.

## Conversations

`ConversationScreen` owns message presentation, the list, composer and message actions page. Use it inside a `Navigator`; no extra route or provider is needed. Supply `ConversationMessage` objects with an `id`, `timestamp` and text or an image. Ink handles keys, rendering and reply previews.

```tsx
<ConversationScreen
  title="Alex"
  messages={messages}
  draft={draft}
  onDraftChange={setDraft}
  actions={message => [{
    label: "React ❤️",
    onPress: () => toggleReaction(message.id),
  }]}
  onSend={({ text, replyTo }) => sendMessage(text, replyTo?.id)}
/>
```

Long-pressing a message or its image opens its preview and the built-in Reply action. Reply returns to the chat, selects the reply target and scrolls to the bottom. Ink builds the preview from the message's author and text (or “Photo” for an image). Outgoing replies use “You”; an omitted incoming author falls back to the screen title. The composer owns the reply banner and its close button. `onSend` receives `{ text, replyTo }`: trimmed text and the original reply target, if selected. After the callback returns, Ink clears the selected reply. The app handles sending, stores any reply snapshot on the sent message and clears its controlled draft when appropriate.

`actions` supplies additional labels and callbacks. Ink returns to the chat before invoking a callback, so it can update data or navigate elsewhere. Back dismisses the actions page without changing the selected reply or chat scroll position. Optional `onRetry`, `onImagePress` and `onDoubleTap` callbacks receive the message. `Message` remains available independently for custom screens.

`timestamp` is milliseconds since the Unix epoch. Ink displays local time for today and adds the month and day for older messages. Set `group` for group chats to show incoming authors; single chats hide authors and outgoing messages never show “You” in their subtitle. Reactions share the subtitle. Optional statuses are `sending`, `sent`, `delivered`, `read` and `failed`; the app supplies them from its messaging service. A failed message offers “Tap to try again” when `onRetry` is supplied. An image supplies `src`, `width` and `height`; a stored `reply` supplies `author` and `text`.

Use `onLoadOlder` and `hasOlder` to prepend history. `onAttach` supplies the composer's plus action. The Single chat and Group chat examples use local data and demonstrate text, image-only messages, replies, reactions and delivery states.

Sending requests keyboard dismissal and the bottom scroll position in the same React update as clearing the reply. The app should add its outgoing message and clear its controlled draft in `onSend`; Ink does not wait for a network acknowledgement. `sending` disables sending, and `loading` shows the initial loading content. Playback and conversation examples have focused emulator verification; real-service integration and physical LP3 workflow validation remain deferred.
