import {
  levelMeter,
  microphonePermission,
  pitchDetector,
} from "@ink/audio";
import { Button, Field, Screen, match } from "ink";

export default function Microphone() {
  const microphone = microphonePermission();
  const level = levelMeter();
  const pitch = pitchDetector();

  return (
    <Screen title="Microphone">
      {match(microphone, {
        loading: () => <Field label="Permission">Checking...</Field>,
        ready: (result) => <Field label="Permission">{result.value}</Field>,
        error: (result) => <Field label="Permission">{result.error.message}</Field>,
      })}
      <Button onPress={() => microphone.request()}>Request Microphone</Button>
      <Field label="Level status">{level.status}</Field>
      {level.status === "error" ? (
        <Field label="Level error">{level.error.message}</Field>
      ) : (
        <Field label="Level">RMS {level.rms}, peak {level.peak}</Field>
      )}
      <Button onPress={() => level.start()}>Start Meter</Button>
      <Button onPress={() => level.stop()}>Stop Meter</Button>
      <Field label="Pitch status">{pitch.status}</Field>
      {pitch.status === "error" ? (
        <Field label="Pitch error">{pitch.error.message}</Field>
      ) : (
        <Field label="Pitch">
          {pitch.note}{pitch.octave}, {pitch.frequencyHz} Hz, {pitch.cents} cents
        </Field>
      )}
      <Button onPress={() => pitch.start()}>Start Tuner</Button>
      <Button onPress={() => pitch.stop()}>Stop Tuner</Button>
    </Screen>
  );
}
