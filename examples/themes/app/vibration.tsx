import { useState } from "react";
import { Screen, SettingsChoices } from "ink";

const options = ["Off", "Light", "Default", "Strong"].map(label => ({ label, value: label }));

export default function Choice() {
  const [value, setValue] = useState("Default");

  return <Screen title="Swipe vibration">
    <SettingsChoices options={options} value={value} onChange={setValue} />
  </Screen>;
}
