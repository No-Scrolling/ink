import { files } from "@ink/files";
import { savedRecording } from "../../../data/recording";
import { usePlayer } from "@ink/audio";
import { Button, Field, Screen, useAction, useRouteParams, useSnapshot } from "ink";
import track from "../../../assets/audio/cant_help.mp3";

export default function LocalPlayback() {
  const params = useRouteParams();
  const session = params.session === "secondary" ? "secondary" : "main";
  const player = usePlayer({ session, usage: "music", mode: "detached" });
  const command = useAction((run: () => Promise<void>) => run());
  const recording = useSnapshot(savedRecording);
  const disabled = !player.state.ready;

  return (
    <Screen title={session === "secondary" ? "Second Player" : "Local Playback"}>
      <Field label="Session">{session}</Field>
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
        <Field label="Track">
          {player.state.current ? `${player.state.index}: ${player.state.current.title}` : "None"}
        </Field>
      )}
      <Field label="Position at last state change">
        {player.state.position} / {player.state.duration} ms
      </Field>
      <Button
        disabled={disabled}
        onPress={() =>
          command.run(async () => {
            await player.setQueue([
              { id: "first", src: track, title: "First Play", artist: "EDEN" },
              { id: "second", src: track, title: "Second Play", artist: "EDEN" },
            ]);
            await player.play();
          })
        }
      >
        Play Local Queue
      </Button>
      <Button
        disabled={disabled || recording.status !== "ready" || !recording.data}
        onPress={() =>
          command.run(async () => {
            const id = await savedRecording.get();
            const file = id ? await files.open(id) : null;
            if (!file) throw new Error("The saved recording is no longer available");
            await player.setQueue([
              { id: file.id, src: file.src, title: file.name, duration: file.duration },
            ]);
            await player.play();
          })
        }
      >
        Play Saved Recording
      </Button>
      <Button disabled={disabled} onPress={() => command.run(player.toggle)}>
        Play / Pause
      </Button>
      <Button disabled={disabled} onPress={() => command.run(player.previous)}>
        Previous Track
      </Button>
      <Button disabled={disabled} onPress={() => command.run(player.next)}>
        Next Track
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
