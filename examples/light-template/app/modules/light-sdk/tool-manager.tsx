import { Button, Field, Screen, useAction, useSnapshot } from "ink";
import { setSharedFilesTask, sharedFiles, useSharedFiles } from "@ink/lightos/tool-manager";
import { toolManagerResult } from "../../../data/background";
import { toolManagerChange } from "../../../workers";

export default function ToolManager() {
  const { files, loading, error } = useSharedFiles("imports");
  const handled = useSnapshot(toolManagerResult);
  const write = useAction(() => sharedFiles.writeText("imports/from-ink.txt", "Written by Ink\n"));
  const background = useAction(() => setSharedFilesTask(toolManagerChange));

  return (
    <Screen title="Tool Manager">
      <Button onPress={background.run}>Enable handler</Button>
      {background.status === "error" && <Field label="Error">{background.error.message}</Field>}
      {handled.status === "ready" && <>
        <Field label="Background changes">{handled.data.changes}</Field>
        <Field label="Last handled files">{handled.data.files}</Field>
      </>}
      <Button onPress={write.run}>Write a file</Button>
      {loading && <Field label="Files">Loading…</Field>}
      {error && <Field label="Error">{error.message}</Field>}
      {write.status === "error" && <Field label="Write error">{write.error.message}</Field>}
      {files.map(file => <Field key={file.path} label={file.name}>{file.size} bytes</Field>)}
    </Screen>
  );
}
