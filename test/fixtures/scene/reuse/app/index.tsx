import { useState } from "react";
import { Button, Screen, Stack, Text, navigate } from "ink";

export default function Counter() {
  const [count, setCount] = useState(0);
  return (
    <Screen title="Counter" centered>
      <Stack gap={16} align="center">
        <Text size={40}>Count: {count}</Text>
        <Button onPress={() => setCount((value) => value + 1)}>Increase</Button>
        <Button onPress={() => navigate("/list")}>Open list</Button>
      </Stack>
    </Screen>
  );
}
