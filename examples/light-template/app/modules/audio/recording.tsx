import { RecordingDuration, useRecorder } from "@ink/audio/recording";
import { Button, Field, Screen, useAction } from "ink";

export default function Recording() {
  const recorder = useRecorder({ updates: "status" });
  const command = useAction((run: () => Promise<void>) => run());
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
      <Button disabled={disabled} onPress={() => command.run(recorder.start)}>Start Recording</Button>
      <Button disabled={disabled} onPress={() => command.run(recorder.stop)}>Save Recording</Button>
      <Button disabled={disabled} onPress={() => command.run(recorder.cancel)}>Cancel Recording</Button>
      <Button disabled={disabled} onPress={() => command.run(recorder.delete)}>Delete Recording</Button>
      {command.status === "error" && <Field label="Command error">{command.error.message}</Field>}
    </Screen>
  );
}
