import { useState } from "react";
import { Screen, SettingsChoices } from "ink";

const options = ["Off", "50 ms", "100 ms", "150 ms", "200 ms", "250 ms"].map(label => ({ label, value: label }));

export default function ContinueAfterLifting() {
  const [value, setValue] = useState("250 ms");

  return <Screen title="Continue after lifting">
    <SettingsChoices options={options} value={value} onChange={setValue} />
  </Screen>;
}
