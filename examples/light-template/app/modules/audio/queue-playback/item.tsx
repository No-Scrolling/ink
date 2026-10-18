import * as v from "valibot";
import { back, Button, EmptyState, replace, Screen, Text, useAction, useRouteParams } from "ink";
import { useQueue } from "./_layout";

export default function QueueItem() {
  const { id } = useRouteParams(v.parser(v.object({ id: v.fallback(v.string(), "") })));
  const { player, ready } = useQueue();
  const index = player.state.queue.findIndex((item) => item.id === id);
  const episode = player.state.queue[index];
  const action = useAction(async (operation: "play" | "up" | "down" | "remove") => {
    if (operation === "play") {
      await player.play(id);
      replace("/modules/audio/queue-playback/player");
      return;
    }
    if (operation === "remove") await player.remove(id);
    else {
      const state = await player.getState();
      const index = state.queue.findIndex((item) => item.id === id);
      if (index < 0) throw new Error("This episode is no longer queued");
      await player.move(id, index + (operation === "up" ? -1 : 1));
    }
    back();
  });
  if (!episode)
    return (
      <Screen title="Episode">
        {ready && <EmptyState title="This episode is no longer queued" />}
      </Screen>
    );
  return (
    <Screen title={episode.title}>
      <Button onPress={() => action.run("play")}>Play</Button>
      <Button disabled={index === 0} onPress={() => action.run("up")}>
        Move up
      </Button>
      <Button disabled={index === player.state.queue.length - 1} onPress={() => action.run("down")}>
        Move down
      </Button>
      <Button href={{ path: "/modules/audio/queue-playback/rename", params: { id } }}>
        Rename
      </Button>
      <Button onPress={() => action.run("remove")}>Remove</Button>
      {action.status === "error" && <Text>{action.error.message}</Text>}
    </Screen>
  );
}
