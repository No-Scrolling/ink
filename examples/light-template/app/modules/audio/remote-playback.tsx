import { usePlayer } from "@ink/audio";
import { Button, Field, Screen, useAction } from "ink";

export default function RemotePlayback() {
  const player = usePlayer({ session: "speech", usage: "speech", mode: "detached" });
  const command = useAction((run: () => Promise<void>) => run());
  const disabled = !player.state.ready;

  return (
    <Screen title="Remote Playback">
      <Field label="Status">
        {!player.state.ready
          ? "Connecting"
          : player.state.buffering
            ? "Buffering"
            : player.state.playing
              ? "Playing"
              : player.state.current
                ? "Paused"
                : "Idle"}
      </Field>
      {player.state.error ? (
        <Field label="Error">{player.state.error.message}</Field>
      ) : (
        <Field label="Track">{player.state.current?.title ?? "None"}</Field>
      )}
      <Field label="Position at last state change">
        {player.state.position} / {player.state.duration} ms
      </Field>
      <Button
        disabled={disabled}
        onPress={() =>
          command.run(async () => {
            await player.setQueue([
              {
                id: "dawn-of-everything",
                src: "https://commons.wikimedia.org/wiki/Special:Redirect/file/Wikipedia_-_The_Dawn_of_Everything.mp3",
                title: "The Dawn of Everything",
                artist: "Wikipedia",
              },
            ]);
            await player.play();
          })
        }
      >
        Play Remote Audio
      </Button>
      <Button disabled={disabled} onPress={() => command.run(player.toggle)}>
        Play / Pause
      </Button>
      <Button disabled={disabled} onPress={() => command.run(() => player.seekBy(-15_000))}>
        Back 15 Seconds
      </Button>
      <Button disabled={disabled} onPress={() => command.run(() => player.seekBy(15_000))}>
        Forward 15 Seconds
      </Button>
      <Button disabled={disabled} onPress={() => command.run(player.stop)}>
        Stop Playback
      </Button>
      {command.status === "error" && <Field label="Command error">{command.error.message}</Field>}
    </Screen>
  );
}
