import { useState } from "react";
import { Screen, Text, TextInput } from "ink";

declare const __inkPost: (source: string) => void;
export default function App() {
  const [value, setValue] = useState("");
  return (
    <Screen title="Kept input fixture">
      <TextInput
        autoFocus
        keepFocus
        value={value}
        onChange={setValue}
        onBlur={(value) => __inkPost(JSON.stringify({ type: "blurred", value }))}
      />
      <Text>Echo: {value}</Text>
    </Screen>
  );
}
