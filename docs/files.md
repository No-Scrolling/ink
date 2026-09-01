---
title: "Files"
description: "Read, write, import, export, and share app files."
tag: "Planned"
---

`@ink/files` manages app-private files and hands files to or from other apps through opaque `FileHandle` values.

## Write an app file

Use `managedFile()` for a file that should survive app restarts.

```tsx
import { managedFile } from "@ink/files";
import { Button, Text, state } from "ink";

const noteText = state("");
const note = managedFile("notes/current.txt");

<Button onPress={() => note.writeText(noteText.value, "text/plain")}>
  Save note
</Button>

{note.status === "error" ? <Text>{note.error.message}</Text> : null}
```

Names are app-relative. They can use `/` for grouping but cannot be absolute, empty, or contain `.` or `..` segments. Declarations with the same name share one app-wide file controller.

`writeText()` writes UTF-8 text and replaces the file atomically. `replace(handle)` copies another file into the managed name. `remove()` deletes the managed file. General app files are limited to 32 MiB.

## Read text

Use `readText()` with a ready handle. The resource defaults to a 1 MiB limit and returns an error for invalid UTF-8.

```tsx
import { readText } from "@ink/files";
import { Text, match } from "ink";

if (note.status === "ready") {
  const contents = readText(note.file);

  return match(contents, {
    loading: () => <Text>Loading note</Text>,
    ready: ({ value }) => <Text>{value}</Text>,
    error: ({ error }) => <Text>{error.message}</Text>,
  });
}
```

`FileHandle` is opaque. It has no filesystem path, Android URI, descriptor, stream, or serialised form. You cannot put it in state, route data, or a store.

## Import a file

Use `fileImporter()` to return a handle, or `textFileImporter()` to validate and read UTF-8 text in one action.

```tsx
import { textFileImporter } from "@ink/files";
import { Button, Text } from "ink";

const importNotes = textFileImporter({
  mimeTypes: ["text/plain", "text/markdown"],
  maxBytes: 1_048_576,
});

<Button onPress={() => importNotes.open()}>Import notes</Button>

{importNotes.status === "ready" ? (
  <Text>{importNotes.value.text}</Text>
) : null}
```

The importer copies the selected content into temporary app storage before returning `ready`. A cancelled picker returns to `idle`. Call `clear()` to release the imported copy.

The handle-only importer defaults to a 10 MiB limit. Its ready value includes `file` and `info`. File information contains `name`, `mimeType`, `sizeBytes`, and `modifiedAtMs`.

## Export a file

Use `fileExporter()` to ask where to save a file.

```tsx
import { fileExporter } from "@ink/files";
import { Button } from "ink";

const exportNote = fileExporter();

{note.status === "ready" ? (
  <Button onPress={() => exportNote.save(note.file, {
    suggestedName: "notes.txt",
  })}>
    Export notes
  </Button>
) : null}
```

The action is `success` after the platform accepts and copies the file. This does not guarantee that another app will keep or process it.

## Share or remove a file

Use `fileSharer().share(file)` to open the Android share sheet. The receiving app gets temporary read access to that file only.

Use `fileRemover(file)` for a module-owned or imported file, then call `run()` to delete it. Removing one handle does not delete a managed copy previously created with `replace()`.

## Lifecycle, permissions, and errors

Managed files survive process death and app upgrades until you remove them or uninstall the app. Imported files may be removed after `clear()`, process exit, or storage pressure.

`readText()` is screen-scoped. Import, export, and share surfaces continue through a temporary app pause and settle when the app resumes. Only one file surface can be open at a time.

The package requests no broad storage permission. The system picker grants access only to the selected source or destination. Sharing grants temporary access only to the receiving app.

Errors distinguish missing files, denied access, interrupted transfers, size limits, invalid text, missing handlers, storage failures, unavailable file surfaces, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.
