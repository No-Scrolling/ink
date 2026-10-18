import { createContext, useContext, useEffect } from "react";
import { usePlayer } from "@ink/audio";
import { Slot, useAction, type Action } from "ink";
import { demoEpisodes, queueInitialised, queueSession } from "../../../../data/audio-queue";

type QueueSession = {
  player: ReturnType<typeof usePlayer>;
  initialise: Action<void, []>;
  ready: boolean;
};

const QueueContext = createContext<QueueSession | null>(null);

export default function QueueLayout() {
  const player = usePlayer({ session: queueSession, mode: "detached" });
  const initialise = useAction(async () => {
    if (!(await queueInitialised.get())) {
      if (!player.state.queue.length)
        await player.setQueue(demoEpisodes.slice(0, 3), { prepare: false });
      await queueInitialised.set(true);
    }
  });
  useEffect(() => {
    if (player.state.ready && initialise.status === "idle") initialise.run();
  }, [player.state.ready, initialise.status, initialise.run]);

  const queue = {
    player,
    initialise,
    ready: player.state.ready && initialise.status === "success",
  };
  return (
    <QueueContext.Provider value={queue}>
      <Slot />
    </QueueContext.Provider>
  );
}

export function useQueue() {
  const queue = useContext(QueueContext);
  if (!queue) throw new Error("Queue pages belong in the queue layout");
  return queue;
}
