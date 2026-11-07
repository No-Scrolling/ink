import { useState } from "react";
import { Button, Screen, TextInput } from "ink";
import { search } from "ink/icons";

export default function Inputs({ hitboxes = false }: { hitboxes?: boolean }) {
  const [query, setQuery] = useState("Edinburgh");
  const [name, setName] = useState("");
  const [note, setNote] = useState("Meet at the station.\nBring the tickets.");
  return <Screen title="Text inputs" debugHitboxes={hitboxes}>
    <TextInput label="Search" value={query} onChange={setQuery} action="search"
      leadingIcon={search} hitSlop={{ top: 8, bottom: 8 }} />
    <TextInput label="Name" value={name} onChange={setName} action="done"
      placeholder="Enter your name" error="Enter a name to continue." />
    <TextInput label="Account" value="alex@example.com" onChange={() => {}} readOnly />
    <TextInput label="Unavailable" value="Disabled input" onChange={() => {}} disabled />
    <TextInput label="Note" value={note} onChange={setNote} action="return" minLines={3} maxLines={5}
      description="Grows to five lines, then scrolls." />
    <Button hitSlop={8} onPress={() => {}}>Save</Button>
  </Screen>;
}
