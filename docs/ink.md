---
title: "Build with Ink"
description: "Small TypeScript apps with a native Light Phone interface."
---

Build Light Phone III apps with React and TypeScript. Ink provides native components, navigation, rendering and device APIs.

Start with [Create an app](/standalone), or run the template below.

## Run the template

From the Ink repository with the Android toolchain installed:

```sh
bun install
cargo run -p ink-cli -- -C examples/light-template check
cargo run -p ink-cli -- -C examples/light-template dev --device emulator-5554 --once
```

Use `ink devices` to find the serial of a connected LP3 or emulator. The template contains `App.tsx`, `ink.toml` and TypeScript configuration. `ink dev` installs the development host and launches the app; omit `--once` to watch for changes. Compatible edits use JavaScript reload or refresh, while native changes rebuild the APK. See [the development loop](/development) and [standalone setup](/standalone).

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

`Screen` provides the title bar, back button, content padding and space above tabs. It scrolls when content overflows. `Stack` arranges children vertically; use `axis="horizontal"` for a row. Set `gap`, `align` and `justify` to control the layout.

Ink uses Public Sans, strong contrast, clear text and a small set of familiar controls. Sizes are Ink logical units. Colours follow the app's light or dark appearance. General icons are imported Material Symbol collections. `Icon.name`, `Button.icon` and `Tab.icon` accept imported icon references; `Icon.size` scales the raster mask, and `tone="muted"` uses the app’s muted colour. Local image/audio files must also be imported; see [assets and native requirements](/build-contracts) for formats and migration.

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

Supply callbacks to fetch data, save choices or navigate. Use `useAction` to track asynchronous work and show errors. Confirmation labels should name the action, such as “Delete list”.

The template includes separate loading, error and confirmation examples.

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

`action="search"` and `action="done"` call `onSubmit`. Navigating closes the keyboard with the page; without a handler, submission dismisses it. Use a dedicated search page and show results on a separate page.

`action="return"` inserts newlines instead of submitting. The input grows to three lines, then scrolls vertically. ConversationScreen uses this mode with a separate send button. `autoFocus` focuses once per screen visit. Ink adjusts the viewport to keep the focused input visible. `Field` displays a value and cannot contain an input.

Update `value` synchronously in `onChange`. Debounce network requests rather than input updates. Ink rejects stale native edits, but cannot decide which delayed app updates you intended to keep.

Use `prefix` and `suffix` for fixed text above the input underline, such as `prefix="£"` and `suffix="GBP"`. They stay visible while the editable text scrolls and are not included in the value. Tap either to focus the input. Keep them short; each is clipped to at most a third of the input width.

Use `inputMode="numeric"` for a digits-only keypad with backspace and a Search or Done action. Values remain strings, preserving leading zeros. Numeric mode rejects non-digit edits and cannot use `action="return"`.

If an app only needs a numpad, import `TextInput` from `ink/input/numeric`. It always uses numeric mode and defaults to Done. Release builds omit letter layouts, emoji data and full-keyboard icons unless the normal `TextInput` or `ConversationScreen` is also used.

```tsx
import { Screen } from "ink";
import { TextInput } from "ink/input/numeric";
```

Import `"@ink/network"` in apps that use `fetch`, `WebSocket` or related web globals. UI imports from `ink` do not install networking globals. Unused image and input components do not add their native capabilities to release builds. Development builds retain the capabilities of loaded modules to support refresh.

Centred images in a full-width vertical stack stay centred on the page when it scrolls; text and controls retain space for the scrollbar.

Placeholders use muted grey. Back dismisses the keyboard, which has a 20-unit bottom inset. Multiline editing keeps the current scroll position unless it needs to reveal the cursor.

Remote images use HTTPS. Images require explicit positive `width` and `height`, can use `fit="contain"` or `"cover"`, and currently have no fallback prop. `bleed` extends an image to the viewport width while preserving the declared aspect ratio. `zoomable` enables gestures; it defaults to false. Arbitrary `file://` images are not accepted by `Image`. Decoding, downsampling, texture caching, pinch zoom and panning stay native. Large media bytes need not pass through JavaScript. For a zoomable image, double-tap cycles through 2×, 3× and 4× magnification, then resets to the fitted image; drag to pan while zoomed. Pinch interaction still needs device verification.

## Collections

Use `.map()` with stable keys for small collections. For large collections, `List` mounts the visible rows with a viewport of extra rows on either side. Row heights are measured automatically; `gap` adds spacing and `followEnd` follows additions only near the end. Stable keys preserve the visible scroll anchor.

For example:

```tsx
<List
  items={songs}
  keyExtractor={song => song.id}
  renderItem={song => <Button onPress={() => play(song)}>{song.title}</Button>}
/>
```

`List` uses its containing screen’s scroll position and preserves the full collection’s scroll extent. Keys must be stable and unique. Rows outside the window unmount, so keep durable row state in the parent or a store. Replace arrays and changed items instead of mutating them. Ink keeps the visible row in place when earlier rows load or change height.

### Load more items

Provide `onLoadMore: () => Promise<void>` and `hasMore`. Ink calls your function as the reader approaches the bottom, allowing one request at a time. Your app fetches and appends items, keeps the pagination cursor, and sets `hasMore` to false when finished.

Normal loading adds no button or spinner. A failed request shows an error and retry action. Short lists can load more pages to fill the screen; a successful request that adds no rows is not repeated at the same boundary. Give the List a new React `key` when switching queries or data sources.

For older history, use `onLoadOlder` and `hasOlder`, then prepend the fetched items. Ink preserves the reader’s position. `ConversationScreen` handles this pattern for chats.

### Follow new items

Set `followEnd` to follow additions while the reader is near the bottom. Following pauses while they scroll or read earlier items.

For a custom history view, `initialEnd` mounts the last rows first but does not scroll the screen itself. `ConversationScreen` coordinates both behaviours.

### Changes to row layout

Use `measurementKey` when a setting changes row layout without changing the items—for example, `measurementKey={textSize}`. Normal item updates, width changes and image loads do not need it.

Use compact item keys and limit loaded history. List keys and content versions have a combined limit of 128 KiB of UTF-8 JSON; exceeding it reports an error.

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

`navigate({ path: "/forecast", params: { placeId } })` or an equivalent `href` opens a screen. Use `back()` to return and `replace()` to replace the current screen. The title-bar button, edge gesture and hardware Back first dismiss the keyboard, then navigate back. Pass IDs and small JSON values as route parameters.

`useRouteParams<T>()` reads route parameters. For external links, supply a `decode(unknown)` function to validate them; a TypeScript generic does not validate runtime data. Unknown notification routes open “Page unavailable” with a back action.

Covered screens keep their React state. Tabs also keep separate scroll positions. Ink pauses screen-owned work while hidden and disposes popped screens. Save data in Store if it must survive an app restart.

## External actions

Import `openURL(url)` and `share({ text })` from `ink`. Both return promises and restore the app when the external window closes. Cancellation resolves normally; invalid URLs, missing handlers and native failures reject. Use [Files and media](/files) to share attachments.

Web links open in Android Custom Tabs. The toolbar follows Ink’s appearance while keeping the site identity and browser security controls visible. If Custom Tabs is unavailable, the command rejects. Telephone links open the dialler; message links open a composer. Other app schemes use their installed handlers. File, content, script and intent URLs are rejected.

## Build and inspect

```sh
ink check
ink info
ink build
ink logs
```

`ink check` validates the project and checks types. `ink info` shows the resolved build configuration. `ink build` creates a signed release APK; `ink dev --once` installs a development build. Check third-party libraries against [runtime compatibility](/runtime-compatibility).

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

Use `Barcode` from `@ink/barcode` for codes. See [Barcodes and passes](/barcode).

## Playing screen

`PlayingScreen` groups optional artwork, title/artists, progress and transport controls at the top of the content area below the header. Its `actions` stay at the bottom and are distributed evenly. Omitting `image` centres the main group vertically between the header and bottom actions; a loading image reserves its space.

Pass playback state and callbacks:

| Props | Purpose |
| --- | --- |
| `playing`, `onPlayPause` | Control playback. |
| `position`, `duration`, optional `onSeek` | Show progress in milliseconds and handle seeking. |
| `title`, optional `onTitlePress` | Display an actionable title. |
| `artists` | Names with optional `onPress` callbacks. |
| `previous`, `next` | Required actions with `onPress`, optional `onLongPress` and `disabled`. Set `seconds` to 5, 10 or 30 for seek icons. |
| `actions` | Bottom controls with `icon`, `onPress`, optional `selected` and `disabled`. |

Your app controls playback and queues. The template’s **Image** and **No Image** examples simulate progress without playing audio.

A long press calls `onLongPress` once and suppresses the tap on release. Moving away cancels it. Bottom actions can change icons and use `selected` for an underline.

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

### Reply and send

Long-press a message or image to open its preview and **Reply** action. Reply returns to the chat and scrolls to the bottom. The banner shows the author and text, or “Photo” for an image. Outgoing replies use “You”; incoming replies without an author use the screen title.

`onSend` receives `{ text, replyTo }`, with trimmed text and the selected message. Ink clears the reply after the callback returns. Your app sends the message, saves any reply snapshot and clears its controlled draft.

`actions` supplies additional labels and callbacks. Ink returns to the chat before invoking a callback, so it can update data or navigate elsewhere. Back dismisses the actions page without changing the selected reply or chat scroll position. Optional `onRetry`, `onImagePress` and `onDoubleTap` callbacks receive the message. `Message` remains available independently for custom screens.

### Message details

`timestamp` is milliseconds since the Unix epoch. Ink shows local time for today and adds the month and day for older messages. Set `group` to show incoming authors in a group chat. Outgoing messages omit the author. Reactions appear on the same line.

Set `status` to `sending`, `sent`, `delivered`, `read` or `failed` using your service’s state. Failed messages offer “Tap to try again” when you supply `onRetry`. Images need `src`, `width` and `height`; saved replies need `author` and `text`.

Use `onLoadOlder` and `hasOlder` to prepend history. `onAttach` supplies the composer's plus action. The Single chat and Group chat examples use local data and demonstrate text, image-only messages, replies, reactions and delivery states.

Sending requests keyboard dismissal and the bottom scroll position in the same React update as clearing the reply. The app should add its outgoing message and clear its controlled draft in `onSend`; Ink does not wait for a network acknowledgement. `sending` disables sending, and `loading` shows the initial loading content.
