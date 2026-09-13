---
title: "Keyboard"
description: "Control focus, editing and spelling."
---

See [Inputs](/text-input) for layouts, action keys and screenshots.

## Choose a keyboard

Ink uses its own keyboard by default. To use the device’s selected Android keyboard for every input:

```toml ink.toml
[keyboard]
provider = "system"
```

Set `provider = "ink"` or omit the section to use Ink’s keyboard. Rebuild the app after changing this setting.

Numeric inputs request a number layout. Search, Done and Return use the corresponding Android keyboard actions. The installed keyboard controls its appearance and suggestions.

System-keyboard builds omit Ink’s keyboard layouts, icons and device spellchecker integration.

## Focus an input

`autoFocus` opens the keyboard once per screen visit:

```tsx
<TextInput value={text} onChange={setText} autoFocus />
```

Ink keeps the cursor visible and preserves multiline scroll position unless it needs to reveal the cursor.

Update `value` immediately in `onChange`. Delay network requests, not text updates.

## Copy, paste and clear

With Ink’s keyboard, long-press plain text for editing actions; long-press again to return.

| Action | Behaviour |
| --- | --- |
| Copy | Copies all text. |
| Paste | Inserts clipboard text at the cursor. |
| Clear | Removes all text. |

Single-line inputs replace pasted line breaks with spaces. Numeric inputs keep only digits. Ink reads the clipboard only when Paste is pressed.

## Spelling

```tsx
<TextInput value={text} onChange={setText} spellCheck autoCorrect />
```

With Ink’s keyboard, `spellCheck` underlines misspelt words. Long-press an underlined word to see up to three suggestions without moving the cursor.

`autoCorrect` replaces likely typos after a space or punctuation. Press Backspace immediately to undo a correction. Ink then leaves that word unchanged for the rest of the editing session.

Both options default to `false`. [Conversations](/conversations) enable them; numeric inputs ignore them. Suggestions use the phone’s spellchecker and language settings, with no bundled dictionary. If the service is unavailable, typing still works.

With the system keyboard, inputs use Android’s text editor for selection and spelling corrections. These options request suggestions and autocorrection from the installed keyboard. Tap an underlined word to see available replacements.

## Close the keyboard

The header Back button leaves the page and closes the keyboard. Android Back closes the keyboard first.

## Numpad-only apps

For apps that only need numbers:

```tsx
import { TextInput } from "ink/input/numeric";
```

It always uses the numpad and defaults to Done. Release builds omit letter layouts, emoji data and full-keyboard icons unless the app also imports the regular `TextInput` or `ConversationScreen`.

## Prefix and suffix

[Prefixes and suffixes](/text-input) stay visible while text scrolls. They are excluded from the value. Tapping either focuses the input. Each is limited to a third of the input width.
