import { Button, EmptyState, Screen } from "ink";
import { demoEpisodes } from "../../../../data/audio-queue";
import { useQueue } from "./_layout";

export default function QueueAdd() {
  const { player, ready } = useQueue();
  const available = ready
    ? demoEpisodes.filter((episode) => !player.state.queue.some((item) => item.id === episode.id))
    : [];
  return (
    <Screen title="Add episode">
      {ready && !available.length && <EmptyState title="All demo episodes are queued" />}
      {available.map((item) => (
        <Button
          key={item.id}
          href={{ path: "/modules/audio/queue-playback/add-episode", params: { id: item.id } }}
        >
          {item.title}
        </Button>
      ))}
    </Screen>
  );
}
