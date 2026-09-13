import { useState } from "react";
import { navigate, Screen, TextInput } from "ink";

export default function Numpad() {
  const [number, setNumber] = useState("");
  return (
    <Screen title="Numpad">
      <TextInput inputMode="numeric" placeholder="Enter a number" value={number} onChange={setNumber}
        action="done"
        onSubmit={value => { if (value) navigate({ path: "/search-results", params: { query: value } }); }} />
    </Screen>
  );
}
