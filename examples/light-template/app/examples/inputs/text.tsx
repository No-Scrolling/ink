import { useState } from "react";
import { navigate, Screen, TextInput } from "ink";

export default function Search() {
  const [query, setQuery] = useState("");
  return (
    <Screen title="Search">
      <TextInput
        placeholder="Search"
        value={query}
        onChange={setQuery}
        action="search"
        onSubmit={(value) => {
          if (value.trim()) navigate({ path: "/search-results", params: { query: value } });
        }}
      />
    </Screen>
  );
}
