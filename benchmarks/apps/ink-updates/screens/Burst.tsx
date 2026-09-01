import { Button, Screen, Stack, Text, state } from "ink";

export default function Burst() {
  const first = state(1);
  const second = state(2);
  const third = state(3);

  return (
    <Screen title="Burst Update" centered>
      <Stack gap={16} align="center">
        <Text>{first.value} / {second.value} / {third.value}</Text>
        <Button
          onPress={() => {
            first.set(first.value + 1);
            second.set(second.value + 1);
            third.set(third.value + 1);
          }}
        >
          Update
        </Button>
      </Stack>
    </Screen>
  );
}
