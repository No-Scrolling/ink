import { Button, EmptyState, ErrorState, navigate, Screen } from "ink";
import { graphicEq, pauseFilled, playArrowFilled } from "ink/icons";
import { useQueue } from "./_layout";

export default function QueuePlayback() {
  const { player, initialise, ready } = useQueue();
  const { state } = player;

  if (initialise.status === "error" && !state.queue.length)
    return (
      <Screen title="Queue">
        <ErrorState message={initialise.error.message} onRetry={initialise.run} />
      </Screen>
    );
  if (ready && !state.queue.length)
    return (
      <Screen title="Queue">
        <EmptyState
          title="Your queue is empty"
          action={{
            label: "Add episode",
            onPress: () => navigate("/modules/audio/queue-playback/add"),
          }}
        />
      </Screen>
    );

  return (
    <Screen
      title="Queue"
      rightAction={{
        icon: graphicEq,
        onPress: () => navigate("/modules/audio/queue-playback/player"),
      }}
    >
      {state.queue.map((item) => (
        <Button
          key={item.id}
          icon={
            item.id === state.current?.id
              ? state.playWhenReady
                ? playArrowFilled
                : pauseFilled
              : undefined
          }
          href={{ path: "/modules/audio/queue-playback/item", params: { id: item.id } }}
        >
          {item.title}
        </Button>
      ))}
      {initialise.status === "error" && (
        <ErrorState message={initialise.error.message} onRetry={initialise.run} />
      )}
      <Button href="/modules/audio/queue-playback/add">Add episode</Button>
      <Button href="/modules/audio/queue-playback/reorder">Reorder</Button>
    </Screen>
  );
}
