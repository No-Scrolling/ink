import { Button, Screen } from "ink";

export default function Audio() {
  return (
    <Screen title="Audio">
      <Button href="/modules/audio/native-playback">Native Playback</Button>
      <Button href="/modules/audio/local-playback">Local Playback</Button>
      <Button href="/modules/audio/remote-playback">Remote Playback</Button>
      <Button href={{ path: "/modules/audio/local-playback", params: { session: "secondary" } }}>
        Second Player
      </Button>
      <Button href="/modules/audio/effects">Effects</Button>
      <Button href="/modules/audio/recording">Recording</Button>
      <Button href="/modules/audio/microphone">Microphone</Button>
    </Screen>
  );
}
