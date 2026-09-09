---
title: "Components"
description: "Choose the right pieces for your screen."
---

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

