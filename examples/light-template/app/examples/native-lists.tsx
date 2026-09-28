import { navigate } from "ink";
import { nativeView } from "ink/view";

type Item = { id: string; label: string; detail: string; text: string; eventCount: number };
const initial = (): Item[] => Array.from({ length: 1000 }, (_, i) => ({
  id: `row-${i}`, label: `Row ${i}`, detail: i % 3 ? "A native row" : "日本語 👩🏽‍🚀 🇯🇵", text: "", eventCount: 0,
}));

export default nativeView(view => {
  const items = view.value(initial());
  const selected = view.value("No selection");
  const summary = view.derive([items], () => `${items.get().length} keyed rows`);
  let added = 0, tick = 0;
  const control = (text: string, onPress: () => void) => view.node("Text", { text, size: 16, width: 90, onPress });
  return view.node("Screen", { title: "Native lists", pinnedHeader: true },
    view.node("Stack", { gap: 12 },
      view.node("Stack", { axis: "horizontal", gap: 10 },
        control("Prepend", () => { const id = `added-${++added}`; items.set([{ id, label: id, detail: "Inserted", text: "", eventCount: 0 }, ...items.get()]); }),
        control("Reverse", () => items.set([...items.get()].reverse())),
        control("Remove", () => items.set(items.get().filter(item => item.id !== selected.get())))),
      view.node("Stack", { axis: "horizontal", gap: 10 },
        control("Refresh", () => { tick++; items.set(items.get().map(item => ({ ...item, label: `${item.id} · ${tick}` }))); }),
        control("Reset", () => { tick = 0; items.set(initial()); }),
        control("Player", () => navigate("/examples/playing/no-image"))),
      view.node("Text", { text: summary, size: 14 }),
      view.node("Text", { text: selected, size: 14 })),
    view.list(items, { key: "id", gap: 12 }, item => view.node("Stack", { gap: 4 },
      view.node("Text", { text: item.at("label"), size: 22, onPress: key => { if (typeof key === "string") selected.set(key); } }),
      view.node("Text", { text: item.at("detail"), size: 12 }),
      view.node("TextInput", { value: item.at("text"), eventCount: item.at("eventCount"), placeholder: "Edit this row",
        onChange: (key, text, eventCount) => {
          if (typeof key === "string" && typeof text === "string" && typeof eventCount === "number") {
            items.set(items.get().map(item => item.id === key ? { ...item, text, eventCount } : item));
          }
        },
      }))),
  );
});
