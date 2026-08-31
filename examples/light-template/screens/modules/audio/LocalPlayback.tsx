import { audioPlayer } from "@ink/audio";
import { Button, Screen, Text } from "ink";

export default function LocalPlayback() {
  const player = audioPlayer({ usage: "music", playback: "detached" });

  return (
    <Screen title="Local Playback">
      <Text>Status: {player.status}</Text>
      {player.status === "error" ? (
        <Text>{player.error.message}</Text>
      ) : player.status === "idle" ? (
        <Text>No Track</Text>
      ) : (
        <Text>Track {player.index}: {player.title}</Text>
      )}
      <Text>{player.positionMs} / {player.durationMs} ms</Text>
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
