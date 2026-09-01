import { Screen, Toggle, state } from "ink";

export default function TogglePaint() {
  const enabled = state(false);

  return (
    <Screen title="Toggle Paint" centered>
      <Toggle
        label="Enabled"
        value={enabled.value}
        onChange={() => enabled.set(!enabled.value)}
      />
    </Screen>
  );
}
