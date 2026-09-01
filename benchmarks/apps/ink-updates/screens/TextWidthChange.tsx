import { Button, Screen, Stack, Text, state } from "ink";

export default function TextWidthChange() {
  const value = state(9);

  return (
    <Screen title="Width-changing Text" centered>
      <Stack gap={16} align="center">
        <Text size={40}>Value: {value.value}</Text>
        <Button onPress={() => value.set(value.value + 1)}>Update</Button>
      </Stack>
    </Screen>
  );
}
