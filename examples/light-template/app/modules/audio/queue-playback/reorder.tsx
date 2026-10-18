import { EmptyState, ReorderList, Screen, Text, useAction } from "ink";
import { useQueue } from "./_layout";

export default function QueueReorder() {
  const { player, ready } = useQueue();
  const move = useAction(player.move);
  return (
    <Screen title="Reorder queue">
      {ready && !player.state.queue.length ? (
        <EmptyState title="Your queue is empty" />
      ) : (
        <ReorderList
          items={player.state.queue}
          keyExtractor={(item) => item.id}
          getLabel={(item) => item.title}
          onChange={(items) => {
            const index = items.findIndex(
              (item, position) => item.id !== player.state.queue[position].id,
            );
            if (index !== -1) move.run(items[index].id, index);
          }}
        />
      )}
      {move.status === "error" && <Text>{move.error.message}</Text>}
    </Screen>
  );
}
