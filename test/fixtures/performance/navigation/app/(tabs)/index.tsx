import { useState } from "react";
import { Button, Screen, Text, navigate } from "ink";

export default function Home() {
  const [saved, setSaved] = useState(false);
  return (
    <Screen title="Library">
      <Text>{saved ? "Saved selection" : "No selection"}</Text>
      <Button onPress={() => setSaved(true)}>Save selection</Button>
      <Button onPress={() => navigate("/album/harbour")}>Open album</Button>
    </Screen>
  );
}
