import { audioPlayer } from "@ink/audio";
import { Button, Field, Screen } from "ink";

export default function LocalPlayback() {
  const player = audioPlayer({ usage: "music", playback: "detached" });

  return (
    <Screen title="Local Playback">
      <Field label="Status">{player.status}</Field>
      {player.status === "error" ? (
        <Field label="Error">{player.error.message}</Field>
      ) : player.status === "idle" ? (
        <Field label="Track">None</Field>
      ) : (
        <Field label="Track">{player.index}: {player.title}</Field>
      )}
      <Field label="Position">{player.positionMs} / {player.durationMs} ms</Field>
      <Button
        onPress={() => {
          player.setQueue([
            {
              id: "first",
              src: "./assets/cant_help.mp3",
              title: "First Play",
              artist: "EDEN",
            },
            {
              id: "second",
              src: "./assets/cant_help.mp3",
              title: "Second Play",
              artist: "EDEN",
            },
          ]);
          player.play();
        }}
      >
        Play Local Queue
      </Button>
      <Button onPress={() => player.playRecording()}>Play Last Recording</Button>
      <Button onPress={() => player.toggle()}>Play / Pause</Button>
      <Button onPress={() => player.previous()}>Previous Track</Button>
      <Button onPress={() => player.next()}>Next Track</Button>
      <Button onPress={() => player.skipBack()}>Back 15 Seconds</Button>
      <Button onPress={() => player.skipForward()}>Forward 15 Seconds</Button>
      <Button onPress={() => player.stop()}>Stop Playback</Button>
    </Screen>
  );
}
