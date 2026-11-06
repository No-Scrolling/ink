import { useState } from "react";
import { Screen, SettingsChoices, Text } from "ink";

declare const __inkPost: (source: string) => void;

export default function Detail() {
  const [value, setValue] = useState("first");
  return <Screen title="Detail">
    <Text>Detail page</Text>
    <SettingsChoices value={value} onChange={next => {
      setValue(next);
      __inkPost(JSON.stringify({ type: "selected", value: next }));
    }} options={[{ value: "first", label: "First" }, { value: "second", label: "Second" }]} />
  </Screen>;
}
