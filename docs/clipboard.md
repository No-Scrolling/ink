---
title: "Clipboard"
description: "Copy and paste short text through the system clipboard."
tag: "Planned"
---

`@ink/clipboard` reads and writes short text through the Android clipboard without continuously observing clipboard contents.

## Copy text

Create one clipboard session and write from a user action:

```tsx
import { clipboard } from "@ink/clipboard";
import { Button, Text } from "ink";

const board = clipboard();

<Button onPress={() => board.writeText("ABCD-1234")}>Copy code</Button>

{board.action.status === "success" ? <Text>Copied</Text> : null}
```

`writeText()` replaces the current plain-text clipboard item. Pass an optional `label` to help Android describe the copied value. Text is limited to 64 KiB.

Ink never records clipboard contents in diagnostics, previews, state restoration, or `app.ink`.

## Paste text

Call `readText()` from a user action. The result remains available only for the lifetime of the declaring screen:

```tsx
<Button onPress={() => board.readText()}>Paste code</Button>

{board.value === null ? null : <Text>{board.value}</Text>}
```

If the clipboard is empty or does not contain text, the successful value is `null`. Ink does not request clipboard contents when the session is created, when a screen opens, or while the app is in the background.

Call `clear()` to clear clipboard text only when the app still owns the current item. Android can prevent an app from removing content written by another app.

## Lifecycle, privacy, and errors

The session is screen-scoped. Leaving the screen clears the value held by Ink. It does not clear the Android clipboard.

Clipboard access requires no runtime permission, but Android can show a privacy notification or deny a background read. Ink therefore permits reads only while the app is foregrounded and after a user action.

Errors distinguish unavailable clipboard access, oversized text, denied reads, unsupported content, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.
