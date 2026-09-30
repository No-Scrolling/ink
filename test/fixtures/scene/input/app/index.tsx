import { useState } from "react";
import { Screen, Text, TextInput } from "ink";

declare const __inkPost: (source: string) => void;
export default function App() {
  const [value, setValue] = useState("");
  return (
    <Screen title="Input fixture">
      <TextInput
        autoFocus
        value={value}
        onChange={setValue}
        onSubmit={(value) => __inkPost(JSON.stringify({ type: "submitted", value }))}
      />
      <Text>Echo: {value}</Text>
    </Screen>
  );
}
