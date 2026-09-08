import { navigate, Screen, TextInput } from "ink";
import { useState } from "react";

export default function SearchScreen() {
  const [query, setQuery] = useState("");
  const submit = () => {
    const value = query.trim();
    if (value) navigate({ path: "/search-results", params: { query: value } });
  };
  return (
    <Screen title="Add location">
      <TextInput value={query} onChange={setQuery} placeholder="Search for a location" action="search" autoFocus onSubmit={submit} />
    </Screen>
  );
}
