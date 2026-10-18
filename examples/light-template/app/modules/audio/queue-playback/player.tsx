import { back, EmptyState, ErrorState, PlayingScreen, Screen, useAction } from "ink";
import { useQueue } from "./_layout";

export default function QueuePlayer() {
  const { player, ready } = useQueue();
  const retry = useAction(player.play);
  const error = player.state.error ?? (retry.status === "error" ? retry.error : null);
  if (error)
    return (
      <Screen title="Playback">
        <ErrorState message={error.message} onRetry={retry.run} />
      </Screen>
    );
  if (!player.state.current)
    return <Screen title="Playback">{ready && <EmptyState title="Your queue is empty" />}</Screen>;
  return (
    <PlayingScreen
      title={player.state.current.title}
      artists={[]}
      playback={player}
      actions={[
        {
          label: "Stop",
          onPress: async () => {
            await player.stop();
            back();
          },
        },
      ]}
    />
  );
}
