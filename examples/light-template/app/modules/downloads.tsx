import * as v from "valibot";
import { downloads } from "@ink/network/downloads";
import { createStore } from "@ink/store";
import { Button, Screen, Text, useAction, useSnapshot } from "ink";

const lastDownload = createStore<string | null>({
  key: "example.download",
  version: 1,
  initial: null,
  decode: v.parser(v.nullable(v.string())),
});

function Progress({ id }: { id: string }) {
  const source = downloads.observe(id);
  const snapshot = useSnapshot(source);
  const action = useAction(async (operation: "pause" | "resume" | "cancel" | "remove") => {
    await downloads[operation](id);
    if (operation === "remove") await lastDownload.reset();
  });
  if (snapshot.status === "loading") return <Text>Loading…</Text>;
  if (snapshot.status === "error") return <Text>{snapshot.error.message}</Text>;
  const value = snapshot.data;
  return (
    <>
      <Text>{value.state}</Text>
      <Text>
        {Math.round(value.received / 1024)} KB
        {value.total === null ? "" : ` / ${Math.round(value.total / 1024)} KB`}
      </Text>
      {value.error && <Text>{value.error}</Text>}
      {(value.state === "running" || value.state === "queued") && (
        <Button onPress={() => action.run("pause")}>Pause</Button>
      )}
      {(value.state === "paused" || value.state === "failed") && (
        <Button onPress={() => action.run("resume")}>Resume</Button>
      )}
      {value.state !== "completed" && value.state !== "cancelled" && (
        <Button onPress={() => action.run("cancel")}>Cancel download</Button>
      )}
      <Button onPress={() => action.run("remove")}>Remove download</Button>
      {action.status === "error" && <Text>{action.error.message}</Text>}
    </>
  );
}

export default function Downloads() {
  const saved = useSnapshot(lastDownload);
  const start = useAction(async () => {
    const value = await downloads.enqueue({
      key: "example",
      url: "http://10.0.2.2:8788/download",
      name: "Example.bin",
    });
    await lastDownload.set(value.id);
  });
  return (
    <Screen title="Downloads">
      <Button onPress={() => start.run()}>Start download</Button>
      {start.status === "error" && <Text>{start.error.message}</Text>}
      {saved.status === "error" && <Text>{saved.error.message}</Text>}
      {saved.status === "ready" && saved.data && <Progress id={saved.data} />}
    </Screen>
  );
}
