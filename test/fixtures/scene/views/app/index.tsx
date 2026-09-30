import { expression, nativeView } from "ink/view";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;
export default nativeView((view) => {
  const count = view.value(0);
  const items = view.collection(
    [
      { id: "a", label: "Alpha" },
      { id: "b", label: "Beta" },
      { id: "c", label: "Gamma" },
    ],
    "id",
  );
  const status = view.compute(expression.concat("Count ", expression.pad(count, 2)));
  view.onMount(() =>
    onNativeMessage("test", (message) => {
      if (message.command === "edit") {
        items.update("b", { label: "Béatrice" });
        items.move("c", "a");
        items.insert({ id: "d", label: "Delta" }, "b");
        items.remove("a");
        count.set(7);
      }
      if (message.command === "reset") {
        items.reset([{ id: "z", label: "Zulu" }]);
        items.update("z", { label: "Zero" });
      }
      if (message.command === "reverse") items.reverse();
    }),
  );
  return view.node(
    "Screen",
    { title: "View fixture" },
    view.node("Text", { text: status }),
    view.node(
      "Button",
      { onPress: () => count.set(count.get() + 1) },
      view.node("Text", { text: "Increase" }),
    ),
    view.list(items, { key: "id", gap: 4 }, (item) =>
      view.node("Text", {
        text: item.at("label"),
        onPress: (...args) => __inkPost(JSON.stringify({ type: "selected", args })),
      }),
    ),
  );
});
