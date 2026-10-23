import { useEffect, useState } from "react";
import { Screen, TextInput } from "ink";
import { onNativeMessage } from "ink/native";
import { boot, post, row } from "./data";
import { JourneyRows } from "./ui";

type LocalItem = ReturnType<typeof row> & { place: string; artist: string };
export function Local() {
  const [catalogue] = useState<readonly LocalItem[]>(() => Array.from({ length: boot.count ?? 5000 }, (_, index) => ({
    ...row(`l${String(index).padStart(5, "0")}`),
    place: index % 2 === 0 ? "Station" : "Harbour", artist: index % 3 === 0 ? "Málaga" : "Zürich",
  })));
  const [query, setQuery] = useState("");
  const [enabled, setEnabled] = useState(false);
  // Keep this ordinary application spelling: the compiler must recognise the joined-field filter.
  const items = enabled ? catalogue.filter(item =>
    [item.place, item.artist].join(" ").toLowerCase().includes(query.toLowerCase()))
    .map(item => ({ ...item, subtitle: `Local ${query}` })) : [row("lseed0", "Local initial")];
  useEffect(() => onNativeMessage("benchmark", message => {
    if (message.command === "report") post({ type: "observed", query, ids: items.map(item => item.id) });
  }), [query, items]);
  return <Screen title="Local catalogue" wide>
    <TextInput value={query} onChange={value => { setEnabled(true); setQuery(value); }} autoFocus placeholder="Search local catalogue" />
    <JourneyRows items={items} />
  </Screen>;
}
