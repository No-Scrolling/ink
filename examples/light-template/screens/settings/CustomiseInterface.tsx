import { Screen, Toggle, state } from "ink";

export default function CustomiseInterface() {
  const invertColours = state(false);

  return (
    <Screen title="Customise Interface">
      <Toggle
        label="Invert Colours"
        value={invertColours.value}
        onChange={() => invertColours.set(!invertColours.value)}
      />
    </Screen>
  );
}
