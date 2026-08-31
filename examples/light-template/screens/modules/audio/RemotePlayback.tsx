import { audioPlayer } from "@ink/audio";
import { Button, Field, Screen } from "ink";

export default function RemotePlayback() {
  const player = audioPlayer({ usage: "speech", playback: "detached" });

  return (
    <Screen title="Remote Playback">
      <Field label="Status">{player.status}</Field>
      {player.status === "error" ? (
        <Field label="Error">{player.error.message}</Field>
      ) : (
        <Field label="Track">{player.title}</Field>
      )}
      <Field label="Position">{player.positionMs} / {player.durationMs} ms</Field>
      <Button
        onPress={() => player.play({
          src: "https://commons.wikimedia.org/wiki/Special:Redirect/file/Wikipedia_-_The_Dawn_of_Everything.mp3",
          title: "The Dawn of Everything",
          artist: "Wikipedia",
        })}
      >
        Play Remote Audio
      </Button>
      <Button onPress={() => player.toggle()}>Play / Pause</Button>
      <Button onPress={() => player.skipBack()}>Back 15 Seconds</Button>
      <Button onPress={() => player.skipForward()}>Forward 15 Seconds</Button>
      <Button onPress={() => player.stop()}>Stop Playback</Button>
    </Screen>
  );
}
