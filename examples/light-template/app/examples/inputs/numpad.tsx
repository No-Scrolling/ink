import { useState } from "react";
import { navigate, Screen } from "ink";
import { TextInput } from "ink/input/numeric";

export default function Numpad() {
  const [number, setNumber] = useState("");
  return (
    <Screen title="Numpad">
      <TextInput placeholder="Enter a number" value={number} onChange={setNumber}
        action="done"
        onSubmit={value => { if (value) navigate({ path: "/search-results", params: { query: value } }); }} />
    </Screen>
  );
}
