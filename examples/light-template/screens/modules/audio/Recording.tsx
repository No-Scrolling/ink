import { audioRecorder } from "@ink/audio";
import { Button, Field, Screen } from "ink";

export default function Recording() {
  const recorder = audioRecorder();

  return (
    <Screen title="Recording">
      <Field label="Status">{recorder.status}</Field>
      {recorder.status === "error" ? (
        <Field label="Error">{recorder.error.message}</Field>
      ) : recorder.status === "ready" ? (
        <Field label="Saved recording">{recorder.recordingDurationMs} ms</Field>
      ) : (
        <Field label="Duration">{recorder.durationMs} ms</Field>
      )}
      <Button onPress={() => recorder.start()}>Start Recording</Button>
      <Button onPress={() => recorder.stop()}>Save Recording</Button>
      <Button onPress={() => recorder.cancel()}>Cancel Recording</Button>
      <Button onPress={() => recorder.delete()}>Delete Recording</Button>
    </Screen>
  );
}
