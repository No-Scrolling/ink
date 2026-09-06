import { useState } from "react";
import { Button, Screen, Stack, Text } from "ink";

export default function Counter() {
  const [count, setCount] = useState(0);

  return (
    <Screen title="Counter" centered>
      <Stack gap={16} align="center">
        <Text size={40}>Count: {count}</Text>
        <Button onPress={() => setCount(count + 1)}>Increase</Button>
      </Stack>
    </Screen>
  );
}

setTimeout(async () => {
  console.log("SPLIT_SMOKE before",typeof Object.getOwnPropertyDescriptor(globalThis,"URL")?.get);
  console.log("SPLIT_SMOKE url",new URL("https://münich.example/?a=1").hostname);
  console.log("SPLIT_SMOKE APIs",new Headers({a:"b"}).get("a"),await new Blob(["hello"]).text(),typeof WebSocket);
  const response = await fetch("https://example.com");
  console.log("SPLIT_SMOKE fetch",response.status,(await response.text()).includes("Example Domain"));
}, 1500);
