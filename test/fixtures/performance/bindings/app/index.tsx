import { expression, nativeView } from "ink/view";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;

export default nativeView((view) => {
  const cells = Array.from({ length: 500 }, () => view.value(0));
  const width = view.value(12);
  view.onMount(() => {
    const stop = onNativeMessage("benchmark", (message) => {
      if (message.command === "small") cells[0].set(Number(message.value));
      if (message.command === "bulk" || message.command === "resize") {
        for (const cell of cells) cell.set(Number(message.value));
        if (message.command === "resize") width.set(message.value === 1 ? 14 : 12);
      }
    });
    __inkPost(JSON.stringify({ type: "ready" }));
    return stop;
  });
  return view.node(
    "Screen",
    { title: "Cells" },
    view.node(
      "Stack",
      { gap: 0 },
      ...Array.from({ length: 25 }, (_, row) =>
        view.node(
          "Stack",
          { axis: "horizontal" },
          ...Array.from({ length: 20 }, (_, column) => {
            const index = row * 20 + column;
            return view.node("Text", {
              width,
              size: 4,
              maxLines: 1,
              text: view.compute(
                expression.concat(String(index).padStart(3, "0"), ":", cells[index]),
              ),
            });
          }),
        ),
      ),
    ),
  );
});
