import { useEffect } from "react";
import { microphone, useLevelMeter, usePitchDetector } from "@ink/audio";
import { Button, Field, Screen, useAction } from "ink";

export default function Microphone() {
  const permission = useAction(microphone.getPermission);
  const request = useAction(async () => {
    await microphone.requestPermission();
    permission.run();
  });
  const level = useLevelMeter();
  const pitch = usePitchDetector();
  const command = useAction((run: () => Promise<void>) => run());
  const busy = command.status === "pending";
  useEffect(() => permission.run(), [permission.run]);

  return (
    <Screen title="Microphone">
      <Field label="Permission">
        {permission.status === "success" ? permission.data
          : permission.status === "error" ? permission.error.message : "Checking..."}
      </Field>
      <Button disabled={request.status === "pending"} onPress={() => request.run()}>Request Microphone</Button>
      {request.status === "error" && <Field label="Permission error">{request.error.message}</Field>}
      <Field label="Level status">{level.ready ? level.state.status : "Connecting"}</Field>
      {level.state.error ? (
        <Field label="Level error">{level.state.error.message}</Field>
      ) : (
        <Field label="Level">RMS {level.state.rms}, peak {level.state.peak}</Field>
      )}
      <Button disabled={!level.ready || busy} onPress={() => command.run(level.start)}>Start Meter</Button>
      <Button disabled={!level.ready || busy} onPress={() => command.run(level.stop)}>Stop Meter</Button>
      <Field label="Pitch status">{pitch.ready ? pitch.state.status : "Connecting"}</Field>
      {pitch.state.error ? (
        <Field label="Pitch error">{pitch.state.error.message}</Field>
      ) : (
        <Field label="Pitch">
          {pitch.state.note}{pitch.state.octave}, {pitch.state.frequency} Hz, {pitch.state.cents} cents
        </Field>
      )}
      <Button disabled={!pitch.ready || busy} onPress={() => command.run(pitch.start)}>Start Tuner</Button>
      <Button disabled={!pitch.ready || busy} onPress={() => command.run(pitch.stop)}>Stop Tuner</Button>
      {command.status === "error" && <Field label="Command error">{command.error.message}</Field>}
    </Screen>
  );
}
