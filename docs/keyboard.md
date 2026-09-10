---
title: "Keyboard"
description: "Focus, editing and spelling options."
---

See [Inputs](/text-input) for keyboard layouts and actions.

## Focus and editing

`autoFocus` focuses the input once per screen visit. Ink keeps the focused input visible. Header Back leaves the page and closes the keyboard; Android Back dismisses the keyboard first.

Update `value` synchronously in `onChange`. Debounce network requests, not input updates.

Long-press the input to show Copy, Paste and Clear. Copy copies all text; Paste inserts at the cursor. Single-line inputs replace pasted line breaks with spaces, and numeric inputs keep only digits. Long-press again to return to the keyboard. Ink reads the clipboard only when Paste is pressed.

Multiline editing preserves the scroll position unless it needs to reveal the cursor.

## Spelling

`spellCheck` underlines misspelt words with suggestions. Long-press an underlined word to show up to three replacements without moving the cursor.

`autoCorrect` corrects likely typos after a space or punctuation. Press Backspace immediately to undo a correction; Ink leaves that word alone for the rest of the editing session.

Both default to false on `TextInput` and are enabled by `ConversationScreen`. Numeric inputs ignore them. They use the device’s enabled spellchecker and language settings, with no bundled dictionary. Typing still works when that service is unavailable.

## Numpad-only apps

Import `TextInput` from `ink/input/numeric` if the app only needs numbers. It always uses numeric mode and defaults to Done.

```tsx
import { TextInput } from "ink/input/numeric";
```

Release builds omit the letter layouts, emoji data and full-keyboard icons unless the regular `TextInput` or `ConversationScreen` is also imported.

## Prefix and suffix

Prefixes and suffixes stay visible while text scrolls and are not part of the value. Tapping either focuses the input. Keep them short: each is limited to a third of the input width.
