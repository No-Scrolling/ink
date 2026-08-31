import { nfcTag } from "@ink/nfc";
import { Button, Screen, Stack, Text, match } from "ink";

export default function Nfc() {
  const tag = nfcTag();

  return (
    <Screen title="NFC">
      {match(tag, {
        loading: () => <Text>Hold an NFC tag near the phone...</Text>,
        ready: (result) => (
          <Stack gap={16}>
            <Text>Serial number: {result.value.serialNumber}</Text>
            {result.value.hasText ? (
              <Text>Text: {result.value.text}</Text>
            ) : (
              <Text>Text: None</Text>
            )}
            {result.value.hasUri ? (
              <Text>URI: {result.value.uri}</Text>
            ) : (
              <Text>URI: None</Text>
            )}
          </Stack>
        ),
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      <Button onPress={() => tag.reload()}>Read another tag</Button>
    </Screen>
  );
}
