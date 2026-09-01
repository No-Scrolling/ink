import { Button, Screen, Stack, Text, state } from "ink";

export default function TextSameWidth() {
  const value = state(7);

  return (
    <Screen title="Same-width Text" centered>
      <Stack gap={16} align="center">
        <Text size={40}>Value: {value.value}</Text>
        <Button onPress={() => value.set(value.value + 1)}>Update</Button>
      </Stack>
    </Screen>
  );
}
