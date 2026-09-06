import { connectivity } from "@ink/connectivity";
import { Button, Field, Screen, Text, useAction, useSnapshot } from "ink";

export default function Connectivity() {
  const connection = useSnapshot(connectivity);
  const refresh = useAction(() => connectivity.get());
  return <Screen title="Connectivity">
    <Field label="Status">{connection.status === "ready" ? connection.data.status : connection.status}</Field>
    {connection.status === "ready" && <>
      <Field label="Transport">{connection.data.transport ?? "Unknown"}</Field>
      <Field label="Metered">{connection.data.metered === undefined ? "Unknown" : connection.data.metered ? "Yes" : "No"}</Field>
    </>}
    <Button onPress={refresh.run}>Refresh connection</Button>
    {connection.status === "error" && <Text>{connection.error.message}</Text>}
  </Screen>;
}
