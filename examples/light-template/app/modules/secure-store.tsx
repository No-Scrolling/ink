import { secureStore } from "@ink/secure-store";
import { Button, Screen, Text, useAction } from "ink";

export default function SecureStore() {
  const write = useAction(() => secureStore.set("template.example", "Example secret"));
  const read = useAction(async () => (await secureStore.get("template.example")) === null ? "No saved secret" : "Saved secret is available");
  const remove = useAction(() => secureStore.remove("template.example"));
  return <Screen title="Secure store">
    <Text>Save a sample secret, reopen the app and check that it is still available.</Text>
    <Button onPress={write.run}>Save sample secret</Button>
    <Button onPress={read.run}>Check saved secret</Button>
    <Button onPress={remove.run}>Remove sample secret</Button>
    {read.status === "success" && <Text>{read.data}</Text>}
    {write.status === "success" && <Text>Secret saved</Text>}
    {remove.status === "success" && <Text>Secret removed</Text>}
    {write.status === "error" && <Text>{write.error.message}</Text>}
    {read.status === "error" && <Text>{read.error.message}</Text>}
    {remove.status === "error" && <Text>{remove.error.message}</Text>}
  </Screen>;
}
