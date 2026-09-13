---
title: "Keyboard"
description: "Control focus, editing and spelling."
---

Ink uses Android’s selected keyboard. See [Inputs](/text-input) for layouts, action keys and screenshots.

## Focus an input

`autoFocus` opens the keyboard once per screen visit:

```tsx
<TextInput value={text} onChange={setText} autoFocus />
```

Ink keeps the cursor visible and preserves multiline scroll position unless it needs to reveal the cursor.

Update `value` immediately in `onChange`. Delay network requests, not text updates.

## Editing

Inputs use Android’s text editor for selection, copy, paste and spelling corrections. Single-line inputs replace pasted line breaks with spaces. Numeric inputs keep only digits.

## Spelling

```tsx
<TextInput value={text} onChange={setText} spellCheck autoCorrect />
```

`spellCheck` requests spelling suggestions. `autoCorrect` requests autocorrection and automatic sentence capitals. Both default to `false`; [Conversations](/conversations) enable them. Numeric inputs ignore them.

The selected keyboard controls which features are available. Tap an underlined word to see replacement suggestions. Ink does not bundle a keyboard or dictionary.

## Action keys

Search, Done and Return request the corresponding Android keyboard action. The selected keyboard controls the key’s appearance.

## Close the keyboard

The header Back button leaves the page and closes the keyboard. Android Back closes the keyboard first.

## Numeric inputs

For inputs that accept only digits:

```tsx
import { TextInput } from "ink/input/numeric";
```

This requests a number layout and defaults to Done. It uses the same Android editor as the regular `TextInput`.

## Prefix and suffix

[Prefixes and suffixes](/text-input) stay visible while text scrolls. They are excluded from the value. Tapping either focuses the input. Each is limited to a third of the input width.
