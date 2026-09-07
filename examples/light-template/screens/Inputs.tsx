import { useState } from "react";
import { Button, Screen, TextInput } from "ink";

export default function Inputs() {
  return (
    <Screen title="Inputs">
      <Button href="/examples/inputs/text">Text</Button>
      <Button href="/examples/inputs/numpad">Numpad</Button>
      <Button href="/examples/inputs/affixes">Prefix and suffix</Button>
    </Screen>
  );
}

export function Affixes() {
  const [amount, setAmount] = useState("");
  return (
    <Screen title="Prefix and suffix">
      <TextInput value={amount} onChange={setAmount} placeholder="Amount"
        prefix="£" suffix="GBP" inputMode="numeric" action="done" />
    </Screen>
  );
}
