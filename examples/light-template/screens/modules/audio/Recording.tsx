import { audioRecorder } from "@ink/audio";
import { Button, Screen, Text } from "ink";

export default function Recording() {
  const recorder = audioRecorder();

  return (
    <Screen title="Recording">
      <Text>Status: {recorder.status}</Text>
      {recorder.status === "error" ? (
        <Text>{recorder.error.message}</Text>
      ) : recorder.status === "ready" ? (
        <Text>Saved: {recorder.recordingDurationMs} ms</Text>
      ) : (
        <Text>Duration: {recorder.durationMs} ms</Text>
      )}
      <Button onPress={() => recorder.start()}>Start Recording</Button>
      <Button onPress={() => recorder.stop()}>Save Recording</Button>
      <Button onPress={() => recorder.cancel()}>Cancel Recording</Button>
      <Button onPress={() => recorder.delete()}>Delete Recording</Button>
    </Screen>
  );
}
