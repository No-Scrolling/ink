import { Button, Column, Screen, Text, state } from "ink";

export default function Counter() {
  const count = state(0);

  return (
    <Screen>
      <Column gap={16}>
        <Text size={32}>Count: {count.value}</Text>
        <Button onPress={() => count.set(count.value + 1)}>Increase</Button>
      </Column>
    </Screen>
  );
}
