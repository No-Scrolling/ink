import { nfcTag } from "@ink/nfc";
import { Button, Screen, Stack, Text } from "ink";

export default function Nfc() {
  const tag = nfcTag();

  return (
    <Screen title="NFC">
      {tag.status === "ready" ? (
        <Stack gap={16}>
          <Text>Serial number: {tag.value.serialNumber}</Text>
          {tag.value.hasText ? (
            <Text>Text: {tag.value.text}</Text>
          ) : (
            <Text>Text: None</Text>
          )}
          {tag.value.hasUri ? (
            <Text>URI: {tag.value.uri}</Text>
          ) : (
            <Text>URI: None</Text>
          )}
        </Stack>
      ) : tag.status === "error" ? (
        <Text>{tag.error.message}</Text>
      ) : (
        <Text>Hold an NFC tag near the phone...</Text>
      )}
      <Button onPress={() => tag.reload()}>Read another tag</Button>
    </Screen>
  );
}
