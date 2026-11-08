---
title: "Tool Manager"
description: "Share app folders with the LightOS Tool Manager and process uploaded files."
---

Expose folders in LightOS’s browser-based Tool Manager with `@ink/lightos/tool-manager`. Your users can upload, download, rename and delete files without opening your app.

## Declare your folders

Add your folders to `ink.toml`:

```toml
[tool_manager]
title = "Music"

[[tool_manager.folders]]
label = "Tracks"
path = "tracks"
```

LightOS lists your app under `title` and creates each folder when it first reads your manifest. Files live in your app’s private `files/shared` directory. You can create folders earlier with `sharedFiles.mkdir`.

Use a unique relative segment for each folder’s `path`, without `/`, `.` or `..`. You can set these options on each folder:

- `header_text`: show instructions above the file list.
- `read_only`: block changes through Tool Manager. Defaults to `false`.
- `show_hidden_files`: include dot-prefixed files. Defaults to `false`.

Ink includes the `light-sdk-tool-manager` capability when you declare folders. Importing the API without this configuration fails the build. Apps that do not opt in include no Tool Manager provider or protocol dependency.

## Show a live file list

Use `useSharedFiles(path?)` to read a folder and refresh it after remote changes or local `sharedFiles` writes:

```tsx
import { Field, Screen } from "ink";
import { useSharedFiles } from "@ink/lightos/tool-manager";

export default function Tracks() {
  const { files, loading, error } = useSharedFiles("tracks");
  return (
    <Screen title="Tracks">
      {loading && <Field label="Files">Loading…</Field>}
      {error && <Field label="Error">{error.message}</Field>}
      {files.map(file => <Field key={file.path} label={file.name}>{file.size} bytes</Field>)}
    </Screen>
  );
}
```

Omit the path to list the shared root. Each `SharedFile` has `name`, `path`, `uri`, `isFile`, `size` in bytes and `lastModified` in milliseconds. The content URI is for LightOS access. Use `sharedFiles` to read files in your app.

LightOS groups remote changes with a three-second debounce. Local writes refresh the hook but do not run your background change handler.

![The shared folder lists two uploaded tracks.](/images/tool-manager/changes.png)

## Read and write shared files

`sharedFiles` uses a separate directory from files managed by `@ink/files`:

```ts
import { sharedFiles } from "@ink/lightos/tool-manager";

await sharedFiles.mkdir("tracks");
await sharedFiles.writeText("tracks/playlist.m3u", "song.flac\n");
const text = await sharedFiles.readText("tracks/playlist.m3u");
const entries = await sharedFiles.list("tracks");
const exists = await sharedFiles.exists("tracks/playlist.m3u");
await sharedFiles.remove("tracks/playlist.m3u");
```

Missing directories and files reject reads. `remove` returns `false` for a missing file or a non-empty directory. Paths outside shared storage, including symbolic links to other directories, are rejected.

### Handle binary files

```ts
import { sharedFiles } from "@ink/lightos/tool-manager";

await sharedFiles.writeBytes("tracks/sample.bin", new Uint8Array([0, 128, 255]));
const bytes = await sharedFiles.readBytes("tracks/sample.bin");
const nextChunk = await sharedFiles.readBytes("tracks/recording.flac", {
  offset: 524_288,
  length: 524_288,
});
```

Byte reads and writes accept at most 512 KiB per call. Pass an offset to read larger files in chunks, and an optional `signal` to cancel a read. Text reads accept files up to 1 MiB and remain subject to Ink’s JSON message limit.

To copy a larger file selected with `@ink/files`, call `sharedFiles.copyFrom("tracks/song.flac", file.src)`. This streams managed `ink-file://` sources and readable `content://` URIs. You can pass `{ signal }` as the third argument to cancel the copy.

Writes replace the file atomically and update its modified time. A failed write leaves the previous file intact.

## Process changes in the background

Include your [worker file](/background) in `ink.toml`:

```toml
[background]
entry = "./workers.ts"
```

Define a task in `workers.ts`. Change handlers receive `null` and reconcile the current folder:

```ts
import * as v from "valibot";
import { defineTask } from "@ink/background";
import { sharedFiles } from "@ink/lightos/tool-manager";

export const processTracks = defineTask({
  id: "music.process-tracks",
  decode: v.parser(v.null()),
  async run({ signal }) {
    signal.throwIfAborted();
    const files = await sharedFiles.list("tracks");
    console.log(`Found ${files.length} tracks`);
    return { status: "success" };
  },
});
```

Register the task from your app, as you would a push handler:

```ts
import { setSharedFilesTask } from "@ink/lightos/tool-manager";
import { processTracks } from "./workers";

await setSharedFilesTask(processTracks);
// Clear the handler when you no longer need it:
await setSharedFilesTask(null);
```

Registration survives app restarts and remains until you clear it. Android schedules the task after remote changes even when your UI is closed. Pending changes combine under one task key; a change during a running task schedules another run.

Each run has Ink’s two-minute limit and cancellation signal. Return `retry` for recoverable failures, and tolerate repeated execution. Android may delay tasks. A force-stopped app must be opened again before Android can run its jobs.

## Connect LightOS

LightOS discovers your exported `${applicationId}.lightfileprovider` provider through its `com.thelightphone.toolmanager.TOOL_MANAGER_PROVIDER` metadata. It calls `get_manifest`, then uses content-provider queries and file descriptors to transfer files.

The provider accepts only Android’s system UID, matching upstream Light SDK. It does not request storage permission or grant other apps access. Run the LightOS emulator as a system app to test it. Ink uses the upstream `client-android` protocol library without the Light SDK client, Compose or WorkManager.

## Test in the emulator

Start the prescribed AVD and select its serial explicitly:

```sh
emulator -avd Light_Phone_III -writable-system
ink dev --device emulator-5554
adb -s emulator-5554 forward tcp:54449 tcp:54449
```

Install the latest LightOS emulator from light-sdk `main` as a system app. Open its **Tool Manager**, then open the displayed URL in your browser. The connection URL contains the session token.

In the Ink template, open **Light SDK > Tool Manager** and select **Enable handler**. Select **Ink Template > Shared files** in the browser and upload a file. Check that the template’s list updates. Close Ink and upload another file, then reopen it to check the background change count. Download and delete a file to check the reverse path.
