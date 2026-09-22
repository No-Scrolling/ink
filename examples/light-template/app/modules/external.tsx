import { Button, Screen, Text, openURL, share, useAction } from "ink";

export default function External() {
  const open = useAction(() => openURL("https://example.com"));
  const send = useAction(() => share({ text: "Shared from Ink" }));
  return <Screen title="External actions">
    <Button onPress={open.run}>Open example website</Button>
    <Button onPress={send.run}>Share sample text</Button>
    {open.status === "error" && <Text>{open.error.message}</Text>}
    {open.status === "success" && <Text>Returned to Ink</Text>}
    {send.status === "error" && <Text>{send.error.message}</Text>}
    {send.status === "success" && <Text>Returned to Ink</Text>}
  </Screen>;
}
