import { Button, Screen, Text, openURL, share, useAction } from "ink";

export default function External() {
  const open = useAction(() => openURL("https://example.com"));
  const send = useAction(() => share({ text: "Shared from Ink" }));
  return <Screen title="External actions">
    <Button onPress={open.run}>Open example website</Button>
    <Button onPress={send.run}>Share sample text</Button>
    {[open, send].map((action, index) => action.status === "error" ? <Text key={index}>{action.error.message}</Text> : action.status === "success" ? <Text key={index}>Returned to Ink</Text> : null)}
  </Screen>;
}
