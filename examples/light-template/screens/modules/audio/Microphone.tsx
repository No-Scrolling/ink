import {
  levelMeter,
  microphonePermission,
  pitchDetector,
} from "@ink/audio";
import { Button, Screen, Text, match } from "ink";

export default function Microphone() {
  const microphone = microphonePermission();
  const level = levelMeter();
  const pitch = pitchDetector();

  return (
    <Screen title="Microphone">
      {match(microphone, {
        loading: () => <Text>Checking Microphone...</Text>,
        ready: (result) => <Text>Permission: {result.value}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      <Button onPress={() => microphone.request()}>Request Microphone</Button>
      <Text>Level: {level.status}</Text>
      {level.status === "error" ? (
        <Text>{level.error}</Text>
      ) : (
        <Text>RMS: {level.rms} Peak: {level.peak}</Text>
      )}
      <Button onPress={() => level.start()}>Start Meter</Button>
      <Button onPress={() => level.stop()}>Stop Meter</Button>
      <Text>Pitch: {pitch.status}</Text>
      {pitch.status === "error" ? (
        <Text>{pitch.error}</Text>
      ) : (
        <Text>{pitch.note}{pitch.octave} {pitch.frequencyHz} Hz {pitch.cents} cents</Text>
      )}
      <Button onPress={() => pitch.start()}>Start Tuner</Button>
      <Button onPress={() => pitch.stop()}>Stop Tuner</Button>
    </Screen>
  );
}
