import { usePlayer } from "@ink/audio";
import { Button, ErrorState, PlayingScreen, Screen, useAction } from "ink";

export default function DashPlayback() {
  const player = usePlayer({ session: "dash", mode: "detached", cache: true });
  const start = useAction(async () => {
    await player.setQueue([
      {
        id: "dig-the-uke",
        src: "https://storage.googleapis.com/shaka-demo-assets/dig-the-uke-clear/dash.mpd",
        title: "Dig the Uke",
        artist: "Shaka demo",
      },
    ]);
    await player.play();
  });
  const { state } = player;
  const error = state.error ?? (start.status === "error" ? start.error : null);

  if (error)
    return (
      <Screen title="DASH playback">
        <ErrorState message={error.message} onRetry={start.run} />
      </Screen>
    );
  if (!state.current)
    return (
      <Screen title="DASH playback">
        <Button disabled={!state.ready} onPress={start.run}>
          Play DASH audio
        </Button>
      </Screen>
    );

  return (
    <PlayingScreen
      title={state.current.title}
      artists={[{ name: state.current.artist ?? "Shaka demo" }]}
      playback={player}
      previous={{ kind: "skip", seconds: 10 }}
      next={{ kind: "skip", seconds: 30 }}
      actions={[
        { label: `${state.speed}×`, onPress: () => player.setSpeed(state.speed === 1 ? 1.5 : 1) },
        {
          label: "Restart",
          onPress: async () => {
            await player.seek(0);
            await player.play();
          },
        },
        { label: "Stop", onPress: player.stop },
      ]}
    />
  );
}
