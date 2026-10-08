import { useEffect } from "react";
import { RecordingDuration, useRecorder } from "@ink/audio/recording";
import { savedRecording } from "../../../data/recording";
import { Button, Field, Screen, useAction } from "ink";

export default function Recording() {
  const recorder = useRecorder();
  const command = useAction((run: () => Promise<void>) => run());
  const save = useAction((id: string) => savedRecording.set(id));
  const recordingId = recorder.state.status === "ready" ? recorder.state.recording?.id : undefined;
  useEffect(() => {
    if (recordingId) save.run(recordingId);
  }, [recordingId, save.run]);
  const disabled = !recorder.ready;

  return (
    <Screen title="Recording">
      <Field label="Status">{recorder.ready ? recorder.state.status : "Connecting"}</Field>
      {recorder.state.error ? (
        <Field label="Error">{recorder.state.error.message}</Field>
      ) : recorder.state.status === "ready" ? (
        <Field label="Saved recording">{recorder.state.recording?.duration ?? 0} ms</Field>
      ) : (
        <RecordingDuration recorder={recorder} />
      )}
      <Button disabled={disabled} onPress={() => command.run(recorder.start)}>
        Start Recording
      </Button>
      <Button
        disabled={disabled || recorder.state.status !== "recording"}
        onPress={() =>
          command.run(async () => {
            await recorder.stop();
          })
        }
      >
        Save Recording
      </Button>
      <Button disabled={disabled} onPress={() => command.run(recorder.cancel)}>
        Cancel Recording
      </Button>
      <Button
        disabled={disabled}
        onPress={() =>
          command.run(async () => {
            await recorder.delete();
            await savedRecording.reset();
          })
        }
      >
        Delete Recording
      </Button>
      {save.status === "error" && <Field label="Save error">{save.error.message}</Field>}
      {command.status === "error" && <Field label="Command error">{command.error.message}</Field>}
    </Screen>
  );
}
