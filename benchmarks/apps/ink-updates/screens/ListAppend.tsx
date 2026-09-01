import { Button, Screen, Stack, Text, state } from "ink";

export default function ListAppend() {
  const items = state(["Alpha", "Bravo", "Charlie", "Delta"]);

  return (
    <Screen title="List Append">
      <Button onPress={() => items.append("Echo")}>Append</Button>
      <Stack gap={16}>
        {items.value.map((item) => (
          <Text>{item}</Text>
        ))}
      </Stack>
    </Screen>
  );
}
