import { lightos } from "@ink/lightos";
import { useEffect } from "react";
import { LevelReadout, PitchReadout, useLevelMeter, usePitchDetector } from "@ink/audio/microphone";
import { Button, Field, Screen, useAction } from "ink";

export default function Microphone() {
  const permission = useAction(() => lightos.getPermission("microphone"));
  const request = useAction(async () => {
    await lightos.requestPermission("microphone");
    permission.run();
  });
  const level = useLevelMeter();
  const pitch = usePitchDetector();
  const command = useAction((run: () => Promise<void>) => run());
  useEffect(() => permission.run(), [permission.run]);

  return (
    <Screen title="Microphone">
      <Field label="Permission">
        {permission.status === "success"
          ? permission.data
          : permission.status === "error"
            ? permission.error.message
            : "Checking..."}
      </Field>
      <Button onPress={() => request.run()}>Request Microphone</Button>
      {request.status === "error" && (
        <Field label="Permission error">{request.error.message}</Field>
      )}
      <Field label="Level status">{level.ready ? level.state.status : "Connecting"}</Field>
      {level.state.error ? (
        <Field label="Level error">{level.state.error.message}</Field>
      ) : (
        <LevelReadout meter={level} />
      )}
      <Button disabled={!level.ready} onPress={() => command.run(level.start)}>
        Start Meter
      </Button>
      <Button disabled={!level.ready} onPress={() => command.run(level.stop)}>
        Stop Meter
      </Button>
      <Field label="Pitch status">{pitch.ready ? pitch.state.status : "Connecting"}</Field>
      {pitch.state.error ? (
        <Field label="Pitch error">{pitch.state.error.message}</Field>
      ) : (
        <PitchReadout detector={pitch} />
      )}
      <Button disabled={!pitch.ready} onPress={() => command.run(pitch.start)}>
        Start Tuner
      </Button>
      <Button disabled={!pitch.ready} onPress={() => command.run(pitch.stop)}>
        Stop Tuner
      </Button>
      {command.status === "error" && <Field label="Command error">{command.error.message}</Field>}
    </Screen>
  );
}
