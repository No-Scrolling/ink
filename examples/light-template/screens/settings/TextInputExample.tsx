import { Screen, TextInput, state } from "ink";

export default function TextInputExample() {
  const name = state("");

  return (
    <Screen title="Text Input">
      <TextInput
        autoFocus
        placeholder="Name..."
        value={name.value}
        onChange={(value) => name.set(value)}
        action="done"
      />
    </Screen>
  );
}
