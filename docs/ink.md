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

Use `ink devices` to find the serial of a connected LP3 or emulator. New apps contain `app/index.tsx`, `ink.toml` and TypeScript configuration. `ink dev` installs the development host and launches the app; omit `--once` to watch for changes. Compatible edits use JavaScript reload or refresh, while native changes rebuild the APK. See [the development loop](/development) and [standalone setup](/standalone).

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

Ink uses Public Sans, strong contrast, clear text and a small set of familiar controls. Sizes are Ink logical units. Colours follow the app's light or dark appearance. Import Material Symbols from `ink/icons`. `Icon.name`, `Button.icon` and `Tabs.Screen.icon` accept imported icon references; `Icon.size` scales the raster mask, and `tone="muted"` uses the app’s muted colour. Local image/audio files must also be imported; see [assets and native requirements](/build-contracts) for formats and migration.

| Component | Use |
| --- | --- |
| `Text` | Wrapping text; `size`, `width`, `align`, `maxLines` and `tabularNumbers` control presentation. |
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

Use `width` to give text a fixed column width, such as day labels beside a forecast. Text wraps within that width, limited by the available space.

Use `maxLines={1}` for a label beside controls in a horizontal `Stack`. Without an explicit `width`, the label shrinks to leave room for the controls and truncates with an ellipsis.

Use `tabularNumbers` for changing readings, timers or counters. It gives digits `0`–`9` equal-width spaces, so changing `340` to `350` doesn't shift the text. Other characters keep their usual spacing, and adding another digit still increases the width.

```tsx
<Text tabularNumbers>{frequency.toFixed(1)} Hz</Text>
```

`useAction(operation)` tracks an asynchronous command as `idle`, `pending`, `success` (with `data`) or `error` (with `error`). Call `action.run(...args)` from a control and disable it while pending. Repeated calls while pending are ignored; failures become state rather than unhandled promise rejections. Unmounting the component or hiding it through navigation discards its eventual UI result, but does not cancel the underlying operation.

## Cached data

Use `resource` for asynchronous reads that should survive tab changes. Define resources outside components, then read them with `useSnapshot`:

```tsx
import { resource, useSnapshot, Screen, Text, LoadingState, ErrorState } from "ink";
import { getForecast } from "./api";

const forecast = resource({
  key: (latitude: number, longitude: number, unit: string) =>
    [latitude, longitude, unit],
  load: (latitude, longitude, unit) => getForecast(latitude, longitude, unit),
  staleTime: 60_000,
  refreshInterval: 60_000,
});

function Forecast({ latitude, longitude, unit }: {
  latitude: number; longitude: number; unit: string;
}) {
  const source = forecast(latitude, longitude, unit);
  const result = useSnapshot(source);
  return <Screen title="Forecast">
    {result.status === "loading" ? <LoadingState /> :
      result.status === "error" ? <ErrorState message={result.error.message} onRetry={source.refresh} /> :
      <Text>{result.data.temperature}°</Text>}
  </Screen>;
}
```

Calling a resource selects a stable cache entry without loading data. Subscribing starts the request. Keys are arrays of strings, finite numbers, booleans or `null`; include every argument that changes the result. Each resource definition has its own cache.

`staleTime` is how long a successful result stays fresh, in milliseconds. It defaults to zero. Stale data remains visible while a new subscription refreshes it. `refreshInterval` separately enables polling while subscribed; omit it to disable polling. Both intervals are measured from the last successful load.

A ready snapshot contains `data`, `refreshing` and `refreshError`. A failed background refresh keeps the data and sets `refreshError`; a failed initial load produces an error snapshot. Failures stop polling and delay subscription-triggered retries for one minute. `source.refresh()` bypasses freshness and retry delays, sharing any pending request. It resolves when the request settles; failures appear in the snapshot.

Hidden tabs unsubscribe, stopping polling when no visible consumers remain. Pending requests finish into the cache. Unused entries expire after five minutes without subscribers, or five minutes after pending work finishes. The cache is in memory and does not survive app restarts; use [Store](/store) for persistent data. Resources do not install networking globals—import `@ink/network` when the loader uses `fetch`.

Use `useAction` for commands such as saving or sending, and normal effects for live subscriptions such as microphone capture.

## Common screen patterns

Compose the state and settings patterns inside a `Screen`; `Confirmation` owns its screen. They use the same native text, controls and spacing as other Ink content:

| Component | Interface |
| --- | --- |
| `SettingsChoices` | `options` with stable `value` and `label`, current `value`, `onChange`, and optional `disabled`. The selected choice is underlined. |
| `LoadingState` | A centred message; optional `label` defaults to “Loading…”. |
| `EmptyState` | `title`, optional `description`, and optional `action` containing a label and press handler. |
| `ErrorState` | A centred `message` and bottom retry action: `onRetry`, optional `retryLabel` (default “Try again”) and `disabled`. |
| `Confirmation` | A complete screen: `title`, small text children, `confirmLabel`, `onConfirm`, optional `centered` message layout, and optional pending state and label. One uppercase action is anchored at the bottom; Back cancels. |

Supply callbacks to fetch data, save choices or navigate. Use `useAction` to track asynchronous work and show errors. Confirmation labels should name the action, such as “Delete list”.

Place `LoadingState` or `ErrorState` directly inside `Screen`. Function components and fragments can wrap them; don't put them inside a layout such as `Stack`. A state replaces the screen's content while keeping its title and navigation. Render one state at a time. For an inline status alongside existing content, use `Text` or `EmptyState` instead.

```tsx
<Screen title="Weather">
  {loading ? <LoadingState /> : error ? (
    <ErrorState message="Could not load the weather." onRetry={reload} />
  ) : <Forecast data={weather} />}
</Screen>
```

These components only display state. They don't fetch data or add delays. Keep useful content visible during background refreshes. The template includes separate loading, error and confirmation examples.

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

Use `spellCheck` to underline misspelt words that have suggestions. Long-press an underlined word to show up to three replacements in the keyboard area without moving the cursor. Tap a replacement to use it, or press Back to return to typing.

Long-press elsewhere in the input to show Copy, Paste and Clear in the keyboard area. Long-press the input again to return to the keyboard, from either actions or spelling suggestions. Copy copies all the input's text. Paste inserts plain text at the cursor; single-line inputs replace line breaks with spaces, and numeric inputs keep only digits. Clear empties the input. These actions work without spellchecking. Ink reads the clipboard only when you tap Paste.

Use `autoCorrect` to correct likely typos after a space or punctuation. Ink leaves ambiguous suggestions unchanged. Press backspace immediately after a correction to restore the original word and remove the separator. Ink leaves that word alone for the rest of the editing session.

Both options default to false on `TextInput` and are enabled by `ConversationScreen`. Numeric inputs ignore them. Ink uses the device's enabled spellchecker and language settings; it does not bundle a dictionary. If the service is unavailable, typing continues without spelling assistance.

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

Ink keeps recently displayed decoded images in memory, so returning to a page can reuse them without loading them again. The cache targets 8 MiB and 256 entries, evicting the least recently used off-screen images first. Visible images are protected and can exceed that budget. Images are cached by source, rendered size and fit; use a new source URL when its content changes. The cache ends with the app session.

## Collections

Use `.map()` with stable keys for small collections. For large collections, `List` mounts the visible rows with a viewport of extra rows on either side. Row heights are measured automatically. The default `gap` is 47, matching the spacing between `Screen` children. Set `gap={0}` for rows without gaps, or provide another value for a compact layout. `followEnd` follows additions only near the end. Stable keys preserve the visible scroll anchor.

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

Ink discovers pages in `app/` and generates the entry point inside `.ink/`. Each page exports a React component as its default export. File-based routing is the only supported entry point.

```text
app/
├── _layout.tsx             # Optional shared providers
├── (tabs)/
│   ├── _layout.tsx         # Tab order and icons
│   ├── index.tsx           # /
│   ├── locations.tsx       # /locations
│   └── settings.tsx        # /settings
├── search.tsx              # /search
└── settings/
    └── units.tsx           # /settings/units
```

`index.tsx` uses its directory's path. Parenthesised directories group pages without adding a path segment. Supporting folders are your choice: `components/`, `hooks/` and `lib/` work well for small apps; feature folders can keep related UI and logic together as an app grows. Files beginning with `_` are ignored except `_layout.tsx`.

Use `[id]` for a dynamic path segment, such as `app/album/[id].tsx` or `app/playlist/[id]/edit.tsx`:

```tsx
navigate({ path: "/album/[id]", params: { id: album.id } });

// app/album/[id].tsx
const { id } = useRouteParams("/album/[id]"); // id is a string
```

TypeScript infers parameter names from the path literal and checks required parameters in `navigate()` and `replace()`. Ink also checks them at runtime and converts numeric IDs to strings. Incoming paths such as `/album/123` match the same page; encoded segments are decoded. Static pages win over dynamic ones, so `/playlist/new` opens `new.tsx`. Duplicate route patterns and repeated parameter names are errors. Catch-all and optional segments are not supported. The app handles IDs that refer to missing data.

A layout wraps the pages beneath it. Use `Slot` to render them:

```tsx
// app/_layout.tsx
import { Slot } from "ink";
import { AccountProvider } from "../components/AccountProvider";

export default function Layout() {
  return <AccountProvider><Slot /></AccountProvider>;
}
```

The root layout stays mounted while navigating. Nested layouts share state across their pages and are removed when no page beneath them remains on the back stack. Covered layouts keep their state and pause effects, just like covered pages. Omit a layout when there is no shared behaviour; Ink already handles navigation, fonts and page backgrounds.

A tab layout declares neighbouring pages by name:

```tsx
// app/(tabs)/_layout.tsx
import { Tabs } from "ink";
import { homeFilled, settingsFilled } from "ink/icons";

export default function Layout() {
  return <Tabs>
    <Tabs.Screen name="index" icon={homeFilled} />
    <Tabs.Screen name="settings" icon={settingsFilled} />
  </Tabs>;
}
```

Tab layouts contain neighbouring page files only; nested directories and nested tab navigation are not supported. Put detail pages outside the tab group so they open above the tab bar. Ink reports unsupported directories when the layout renders.

Declare the tabs you want to show, using ordinary React conditions or array mapping. A neighbouring page omitted from `Tabs.Screen` is still reachable: navigating to it opens a separate page without the tab bar, and Back returns to the previous screen. Keep at least one visible tab. Removing the selected tab selects the first remaining tab; reordering tabs preserves the selection and page state. Removing a tab unmounts its tab content.

At startup, if the index page belongs to a tab layout but is omitted from its visible tabs, Ink opens the first visible tab directly. It does not mount the hidden index page or add a redirect to the back stack. Explicit navigation and incoming links to a hidden page still open that page normally. Load saved tab preferences before rendering `Tabs` if they determine the initial selection.

Tab order follows the declarations. A tab group has one place on the back stack. Tab presses, `navigate()` and `replace()` targeting an existing tab group select that tab, preserve its state, and dismiss any pages above the group. They do not create duplicate tab screens. Ordinary pages still create separate instances with `navigate()` and replace the current instance with `replace()`.

Use `Tabs.Action` for a navigation-bar button that runs an action instead of switching tabs:

```tsx
import { Tabs, navigate } from "ink";
import { wallet, photoCamera, settings } from "ink/icons";

export default function Layout() {
  return <Tabs>
    <Tabs.Screen name="index" icon={wallet} />
    <Tabs.Action icon={photoCamera} onPress={() => navigate("/scan")} />
    <Tabs.Screen name="settings" icon={settings} />
  </Tabs>;
}
```

Here, `app/scan.tsx` sits outside the tab group. Scan opens above the tabs; Back returns to the previously selected tab. Actions follow the JSX order, use the unselected icon colour and never become selected. They do not count towards the requirement for at least one tab or affect the startup selection. Give mapped actions stable React keys.

Move existing `App.tsx` screens into `app/` and remove manual route registration. Use an optional `_layout.tsx` for shared providers, `Slot` for its pages and `Tabs.Screen` for tabs.

`navigate({ path: "/forecast", params: { placeId } })` or an equivalent `href` opens a screen. Use `back()` to return and `replace()` to replace the current screen. The title-bar button and a left-edge swipe above the keyboard navigate back directly. Hardware Back dismisses the keyboard first. Pass IDs and small JSON values as route parameters.

`useRouteParams("/album/[id]")` reads named path parameters as strings. The path literal supplies the types without a generated type file; it does not check whether that filename exists. For other parameters, supply a `decode(unknown)` function to validate them. Existing `useRouteParams<T>()` calls remain supported, but a TypeScript generic does not validate runtime data. Unknown or malformed notification routes open “Page unavailable” with a back action.

Covered screens keep their React state. Tabs also keep separate scroll positions. Ink pauses screen-owned work while hidden and disposes popped screens. Save data in Store if it must survive an app restart.

To save a setting and return, await the store update, then call `back()`. Ink applies the returning screen’s current store snapshot before presenting it, so the previous value doesn't briefly appear.

## External actions

Import `openURL(url)` and `share({ text })` from `ink`. Both return promises and restore the app when the external window closes. These imports add the native `external` capability, including browser package queries. Cancellation resolves normally; invalid URLs, missing handlers and native failures reject. Use [Files and media](/files) to share attachments.

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
import { moreHoriz } from "ink/icons";

<Row image={artwork} title="Album title" subtitle="Artist name" href="/album" />
<Screen title="Albums" rightAction={{ icon: moreHoriz, onPress: openActions }}>
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

`ConversationScreen` owns message presentation, the list, composer and message actions page. Use it in an `app/` page; no extra route or provider is needed for message actions. Supply `ConversationMessage` objects with an `id`, `timestamp` and text or an image. Ink handles keys, rendering and reply previews.

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
