import * as v from "valibot";
import type { AudioItem } from "@ink/audio";
import { back, Button, EmptyState, Screen, Text, useAction, useRouteParams } from "ink";
import { demoEpisodes } from "../../../../data/audio-queue";
import { useQueue } from "./_layout";

export default function QueueAddEpisode() {
  const { id } = useRouteParams(v.parser(v.object({ id: v.fallback(v.string(), "") })));
  const { player, ready } = useQueue();
  const episode = demoEpisodes.find((item) => item.id === id);
  const add = useAction(async (episode: AudioItem, at?: "next") => {
    await player.add(episode, { at });
    back();
  });
  if (!episode)
    return (
      <Screen title="Add episode">
        <EmptyState title="Episode not found" />
      </Screen>
    );
  const unavailable = !ready || player.state.queue.some((item) => item.id === id);
  return (
    <Screen title={episode.title}>
      <Button disabled={unavailable} onPress={() => add.run(episode, "next")}>
        Play next
      </Button>
      <Button disabled={unavailable} onPress={() => add.run(episode)}>
        Add to end
      </Button>
      {add.status === "error" && <Text>{add.error.message}</Text>}
    </Screen>
  );
}
