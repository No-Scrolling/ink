import { useState } from "react";
import { Screen, TextInput } from "ink";

export default function Affixes() {
  const [amount, setAmount] = useState("");
  return (
    <Screen title="Prefix and suffix">
      <TextInput value={amount} onChange={setAmount} placeholder="Amount"
        prefix="£" suffix="GBP" inputMode="numeric" action="done" />
    </Screen>
  );
}
