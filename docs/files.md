---
title: "Files"
description: "Own, import, export, and share app files through safe references."
tag: "Planned"
---

`@ink/files` owns app-private files and exchanges files with other apps through opaque handles. Callers never receive filesystem paths, Android URIs, or descriptors.

## Manage an app file

Use `file()` for a durable file that should survive app restarts:

```tsx
import { file } from "@ink/files";
import { Button, Text, state } from "ink";

const noteText = state("");
const note = file("notes/current.txt");

<Button onPress={() => note.writeText(noteText.value, {
  mimeType: "text/plain",
})}>
  Save note
</Button>

{note.status === "ready" ? <Text>{note.info.sizeBytes} bytes</Text> : null}
```

The managed file owns `writeText()`, `replace()`, `export()`, `share()`, and `remove()`. Each operation is an action with `idle`, `running`, `success`, and `error` states. Replacing or writing a file is atomic.

Names are app-relative. They can use `/` for grouping but cannot be absolute, empty, or contain `.` or `..` segments. Declarations with the same name reconnect to the same application-scoped file.

## Manage a text file

Use `textFile()` when reading and writing UTF-8 text is the complete domain operation:

```tsx
import { textFile } from "@ink/files";
import { Text, match } from "ink";

const notes = textFile("notes/current.txt", {
  initial: "",
  maximumBytes: 1_048_576,
});

{match(notes, {
  loading: () => <Text>Loading notes</Text>,
  ready: ({ value }) => <Text>{value}</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}
```

`textFile()` owns opening, UTF-8 validation, atomic writes, and reloads. Use the general `file()` interface for binary content or when another module consumes the file.

## Import a file

Use `filePicker()` to copy a selected file into temporary app storage:

```tsx
const picker = filePicker({
  mimeTypes: ["text/plain", "text/markdown"],
  maximumBytes: 1_048_576,
});

<Button onPress={() => picker.open()}>Import notes</Button>

{picker.status === "success" ? (
  <Button onPress={() => note.replace(picker.value.file)}>
    Keep imported file
  </Button>
) : null}
```

The result contains a temporary `FileHandle` and information including `name`, `mimeType`, `sizeBytes`, and `modifiedAtMs`. Closing the system picker returns the action to `idle`.

Temporary handles belong to their owning action or session. They cannot enter state, route data, Store, or Background work. An operation that accepts a handle acquires a lease before returning. Call `replace()` to copy it into a managed file when it must become durable.

## Use a durable reference

A ready managed file exposes `reference`, an opaque `FileReference` that remains valid across navigation and process death. Downloads also return durable references. Reader, Media, and Background accept these references without gaining path access.

Removing the managed file invalidates its reference. Active consumers finish through their existing lease; later operations receive a `missing` error.

## Export or share a file

Call methods on the managed file:

```tsx
<Button onPress={() => note.export({ suggestedName: "notes.txt" })}>
  Export notes
</Button>
<Button onPress={() => note.share()}>Share notes</Button>
```

Export asks the user where to create a copy. Share opens the Android share sheet and grants the selected receiving app temporary read access. Neither operation guarantees that another app keeps or processes the file.

## Lifecycle, permissions, and errors

Managed files are application-scoped and survive process death and upgrades. Temporary imports are released when their owner is cleared or disposed and can be removed earlier under storage pressure.

System file surfaces continue through a temporary app pause and settle when the app resumes. Only one file surface can be open at a time.

The module requests no broad storage permission. Errors distinguish missing files, denied access, interrupted transfers, size limits, invalid text, unavailable handlers, storage failures, expired handles, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.
