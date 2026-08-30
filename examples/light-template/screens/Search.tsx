import { Screen, TextInput, state } from "ink";

export default function Search() {
  const query = state("");

  return (
    <Screen title="Search">
      <TextInput
        placeholder="Search..."
        value={query.value}
        onChange={(value) => query.set(value)}
        action="search"
      />
    </Screen>
  );
}
