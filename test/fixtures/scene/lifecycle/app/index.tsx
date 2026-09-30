import { Activity, useLayoutEffect, useState } from "react";
import { Screen, Text } from "ink";
import { onNativeMessage } from "ink/native";
import { nativeView, type ViewValue, type ViewCollection } from "ink/view";

declare const __inkPost: (source: string) => void;
const observed = { starts: 0, stops: 0, deliveries: 0, actions: 0 };
let retainedValue: ViewValue<string>;
let retainedItems: ViewCollection<{ id: string; label: string }>;

const NativePage = nativeView(view => {
  const value = view.value("Initial value");
  const items = view.collection([{ id: "first", label: "Initial row" }], "id");
  retainedValue = value;
  retainedItems = items;
  view.onMount(() => {
    observed.starts++;
    const stop = onNativeMessage("source", () => {
      observed.deliveries++;
      value.set(`Delivery ${observed.deliveries}`);
    });
    return () => { observed.stops++; stop(); };
  });
  return view.node("Screen", { title: "Native lifecycle" },
    view.node("Text", { text: value }),
    view.node("Text", {
      text: "Invoke native action",
      onPress: () => { observed.actions++; value.set(`Action ${observed.actions}`); },
    }),
    view.list(items, { key: "id" }, item => view.node("Text", { text: item.at("label") })),
  );
});

export default function Lifecycle() {
  const [mode, setMode] = useState<"visible" | "hidden" | "removed">("visible");
  useLayoutEffect(() => onNativeMessage("test", message => {
    if (message.command === "hide") setMode("hidden");
    if (message.command === "show") setMode("visible");
    if (message.command === "remove") setMode("removed");
    if (message.command === "mutate-hidden") {
      retainedValue.set("Value changed while hidden");
      retainedItems.update("first", { label: "Row changed while hidden" });
      retainedItems.insert({ id: "second", label: "Inserted while hidden" });
    }
    if (message.command === "report") __inkPost(JSON.stringify({ type: "observed", ...observed }));
  }), []);
  return <>
    {mode !== "removed" && <Activity mode={mode}><NativePage /></Activity>}
    {mode !== "visible" && <Screen title="Lifecycle placeholder"><Text>{mode === "removed" ? "Removed" : "Hidden"}</Text></Screen>}
  </>;
}
