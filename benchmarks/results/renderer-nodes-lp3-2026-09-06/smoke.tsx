import { useState } from "react";
import { Button, Image, Screen, Stack, Text } from "ink";
import picture from "./image.jpg";

export default function App() {
  const [count, setCount] = useState(0);
  const [mode, setMode] = useState(0);
  return (
    <Screen title="Rendering check">
      <Stack gap={12} align="center">
        <Text size={mode === 3 ? 28 : 24} align={mode === 3 ? "end" : "center"} maxLines={2}>
          Count: <Text>{count}</Text>
        </Text>
        <Button onPress={() => setCount(count + 1)}>Increase</Button>
        <Button onPress={() => { setMode((mode + 1) % 4); console.log("RENDER_SMOKE mode", (mode + 1) % 4); }}>Next</Button>
        {mode === 1 && <Image src={picture} width={160} height={160} />}
        {mode === 2 && <Text size={48}>😀 🌍</Text>}
        {mode === 3 && <Text>Text resources updated</Text>}
      </Stack>
    </Screen>
  );
}
