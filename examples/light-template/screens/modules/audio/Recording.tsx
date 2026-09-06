import { useRecorder } from "@ink/audio/capture";
import { Button, Field, Screen, useAction } from "ink";

export default function Recording() {
  const recorder = useRecorder();
  const command = useAction((run: () => Promise<void>) => run());
  const disabled = !recorder.ready || command.status === "pending";

  return (
    <Screen title="Recording">
      <Field label="Status">{recorder.ready ? recorder.state.status : "Connecting"}</Field>
      {recorder.state.error ? (
        <Field label="Error">{recorder.state.error.message}</Field>
      ) : recorder.state.status === "ready" ? (
        <Field label="Saved recording">{recorder.state.recording?.duration ?? 0} ms</Field>
      ) : (
        <Field label="Duration">{recorder.state.duration} ms</Field>
      )}
      <Button disabled={disabled} onPress={() => command.run(recorder.start)}>Start Recording</Button>
      <Button disabled={disabled} onPress={() => command.run(recorder.stop)}>Save Recording</Button>
      <Button disabled={disabled} onPress={() => command.run(recorder.cancel)}>Cancel Recording</Button>
      <Button disabled={disabled} onPress={() => command.run(recorder.delete)}>Delete Recording</Button>
      {command.status === "error" && <Field label="Command error">{command.error.message}</Field>}
    </Screen>
  );
}
