import { usePlayer } from "@ink/audio";
import { Button, ErrorState, PlayingScreen, Screen, useAction } from "ink";
import track from "../../../assets/audio/cant_help.mp3";

export default function NativePlayback() {
  const player = usePlayer({ session: "main", mode: "detached" });
  const command = useAction(async (run: () => Promise<void>) => run());
  const { state } = player;
  if (state.error || command.status === "error")
    return (
      <Screen title="Native playback">
        <ErrorState
          message={
            state.error?.message ??
            (command.status === "error" ? command.error.message : "Playback failed")
          }
          onRetry={() => command.run(player.play)}
        />
      </Screen>
    );
  if (!state.current)
    return (
      <Screen title="Native playback">
        <Button
          disabled={!state.ready}
          onPress={() =>
            command.run(async () => {
              await player.setQueue([{ id: "local", src: track, title: "Can't help" }]);
              await player.play();
            })
          }
        >
          Play local audio
        </Button>
      </Screen>
    );
  return (
    <PlayingScreen
      title={state.current.title}
      artists={[]}
      playback={player}
      actions={[
        { label: `${state.speed}×`, onPress: () => player.setSpeed(state.speed === 1 ? 2 : 1) },
      ]}
    />
  );
}
