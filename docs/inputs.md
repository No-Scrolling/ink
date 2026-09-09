---
title: "Inputs and images"
description: "Enter text and display images."
---

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

