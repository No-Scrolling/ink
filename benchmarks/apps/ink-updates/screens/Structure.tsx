import { Button, Screen, Stack, Text, state } from "ink";

export default function Structure() {
  const expanded = state(false);

  return (
    <Screen title="Structural Update" centered>
      <Stack gap={16} align="center">
        {expanded.value ? <Text>Expanded content</Text> : <Text>Collapsed</Text>}
        <Button onPress={() => expanded.set(!expanded.value)}>Update</Button>
      </Stack>
    </Screen>
  );
}
