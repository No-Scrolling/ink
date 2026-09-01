import { Button, Screen, Stack, Text, state } from "ink";

export default function Emoji() {
  const value = state("Plain text");

  return (
    <Screen title="Emoji Cold Path" centered>
      <Stack gap={16} align="center">
        <Text size={32}>{value.value}</Text>
        <Button onPress={() => value.set("☀️ 🚌 ✅")}>Update</Button>
      </Stack>
    </Screen>
  );
}
