import { useState } from "react";
import { Screen, SettingsChoices } from "ink";

const options = ["Off", "Atlas Calibration"].map(label => ({ label, value: label }));

export default function Choice() {
  const [value, setValue] = useState("Atlas Calibration");

  return <Screen title="Calibration profile">
    <SettingsChoices options={options} value={value} onChange={setValue} />
  </Screen>;
}
