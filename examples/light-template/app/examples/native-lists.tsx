import { navigate } from "ink";
import { expression, nativeView } from "ink/view";

type Item = { id: string; label: string; detail: string; text: string; eventCount: number };
const initial = (): Item[] => Array.from({ length: 1000 }, (_, i) => ({
  id: `row-${i}`, label: `Row ${i}`, detail: i % 3 ? "A native row" : "日本語 👩🏽‍🚀 🇯🇵", text: "", eventCount: 0,
}));

export default nativeView(view => {
  const items = view.collection(initial(), "id");
  const selected = view.value("No selection");
  const summary = view.compute(expression.concat(expression.length(items), " keyed rows"));
  let keys = Array.from({ length: 1000 }, (_, i) => `row-${i}`);
  let added = 0, tick = 0;
  const control = (text: string, onPress: () => void) => view.node("Text", { text, size: 16, width: 90, onPress });
  return view.node("Screen", { title: "Native lists", pinnedHeader: true },
    view.node("Stack", { gap: 12 },
      view.node("Stack", { axis: "horizontal", gap: 10 },
        control("Prepend", () => { const id = `added-${++added}`; items.insert({ id, label: id, detail: "Inserted", text: "", eventCount: 0 }, keys[0] ?? null); keys.unshift(id); }),
        control("Reverse", () => { items.reverse(); keys.reverse(); }),
        control("Remove", () => { if (keys.includes(selected.get())) { items.remove(selected.get()); keys = keys.filter(key => key !== selected.get()); } })),
      view.node("Stack", { axis: "horizontal", gap: 10 },
        control("Refresh", () => { tick++; for (const key of keys) items.update(key, { label: `${key} · ${tick}` }); }),
        control("Reset", () => { tick = 0; items.reset(initial()); keys = Array.from({ length: 1000 }, (_, i) => `row-${i}`); }),
        control("Player", () => navigate("/examples/playing/no-image"))),
      view.node("Text", { text: summary, size: 14 }),
      view.node("Text", { text: selected, size: 14 })),
    view.list(items, { key: "id", gap: 12 }, item => view.node("Stack", { gap: 4 },
      view.node("Text", { text: item.at("label"), size: 22, onPress: key => { if (typeof key === "string") selected.set(key); } }),
      view.node("Text", { text: item.at("detail"), size: 12 }),
      view.node("TextInput", { value: item.at("text"), eventCount: item.at("eventCount"), placeholder: "Edit this row",
        onChange: (key, text, eventCount) => {
          if (typeof key === "string" && typeof text === "string" && typeof eventCount === "number") {
            items.update(key, { text, eventCount });
          }
        },
      }))),
  );
});
