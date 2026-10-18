import { useState } from "react";
import * as v from "valibot";
import type { AudioItem } from "@ink/audio";
import { back, EmptyState, Screen, Text, TextInput, useAction, useRouteParams } from "ink";
import { useQueue } from "./_layout";

export default function QueueRename() {
  const { id } = useRouteParams(v.parser(v.object({ id: v.fallback(v.string(), "") })));
  const { player, ready } = useQueue();
  const episode = player.state.queue.find((item) => item.id === id);
  if (!episode)
    return (
      <Screen title="Rename episode">
        {ready && <EmptyState title="This episode is no longer queued" />}
      </Screen>
    );
  return <RenameForm key={episode.id} episode={episode} player={player} />;
}

function RenameForm({
  episode: initialEpisode,
  player,
}: {
  episode: AudioItem;
  player: ReturnType<typeof useQueue>["player"];
}) {
  const [title, setTitle] = useState(initialEpisode.title);
  const rename = useAction(async (value: string) => {
    if (!value.trim()) throw new Error("Enter a title");
    await player.replace(initialEpisode.id, { ...initialEpisode, title: value.trim() });
    back();
  });
  return (
    <Screen title="Rename episode">
      <TextInput value={title} onChange={setTitle} onSubmit={rename.run} action="done" autoFocus />
      {rename.status === "error" && <Text>{rename.error.message}</Text>}
    </Screen>
  );
}
