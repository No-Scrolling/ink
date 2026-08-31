import { nfcTag } from "@ink/nfc";
import { Button, Field, Screen, Stack, match } from "ink";

export default function Nfc() {
  const tag = nfcTag();

  return (
    <Screen title="NFC">
      {match(tag, {
        loading: () => <Field label="Tag">Hold an NFC tag near the phone...</Field>,
        ready: (result) => (
          <Stack gap={16}>
            <Field label="Serial number">{result.value.serialNumber}</Field>
            {result.value.hasText ? (
              <Field label="Text">{result.value.text}</Field>
            ) : (
              <Field label="Text">None</Field>
            )}
            {result.value.hasUri ? (
              <Field label="URI">{result.value.uri}</Field>
            ) : (
              <Field label="URI">None</Field>
            )}
          </Stack>
        ),
        error: (result) => <Field label="Tag">{result.error.message}</Field>,
      })}
      <Button onPress={() => tag.reload()}>Read another tag</Button>
    </Screen>
  );
}
