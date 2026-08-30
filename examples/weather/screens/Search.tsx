import { Button, Screen, TextInput, state } from "ink";

export default function Search() {
  const query = state("");

  return (
    <Screen title="Search">
      <TextInput
        placeholder="Search for a location"
        value={query.value}
        onChange={(value) => query.set(value)}
        action="search"
      />
      <Button>London, England</Button>
    </Screen>
  );
}
