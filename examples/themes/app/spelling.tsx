import { useState } from "react";
import { Screen, SettingsChoices } from "ink";

const options = ["British", "American"].map(label => ({ label, value: label }));

export default function Choice() {
  const [value, setValue] = useState("British");

  return <Screen title="Spelling">
    <SettingsChoices options={options} value={value} onChange={setValue} />
  </Screen>;
}
