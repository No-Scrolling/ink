import { nativeView } from "ink/view";
import { refresh } from "ink/icons";

export default nativeView(view => {
  const count = view.value(100);
  const tick = view.value(0);
  const reverse = view.value(false);
  const hidden = view.value(false);
  const resize = view.value(false);
  const labels = view.derive([count, tick, reverse], () => Array.from({ length: 500 }, (_, index) => {
    const id = reverse.get() ? count.get() - index - 1 : index;
    return `${String(id).padStart(3, "0")}:${(id + tick.get()) % 100}`;
  }));
  const gap = view.derive([tick, resize], () => resize.get() ? tick.get() % 2 : 0);
  const summary = view.derive([count, tick], () => `${count.get()} cells · ${tick.get()} updates`);
  const hiddenRows = view.derive([count], () => Array.from({ length: 50 }, (_, row) => row * 10 >= count.get()));
  const hiddenCells = view.derive([count], () => Array.from({ length: 500 }, (_, id) => id >= count.get()));
  const hiddenLabel = view.derive([hidden], () => hidden.get() ? "Show" : "Hide");
  const resizeLabel = view.derive([resize], () => resize.get() ? "Resize on" : "Resize off");
  return view.node("Screen", {
    title: "Update benchmark", pinnedHeader: true, rightIcon: refresh,
    onRightPress: () => tick.set(tick.get() + 1),
  },
    view.node("Stack", { gap: 47 }, view.node("Stack", { gap: 8 },
      view.node("Stack", { axis: "horizontal", gap: 16 },
        ...[1, 10, 100, 500].map(value => view.node("Text", {
          text: String(value), size: 18, width: 50, onPress: () => { count.set(value); tick.set(0); },
        }))),
      view.node("Stack", { axis: "horizontal", gap: 16 },
        view.node("Text", { text: "Reverse", size: 14, width: 85, onPress: () => reverse.set(!reverse.get()) }),
        view.node("Text", { text: hiddenLabel, size: 14, width: 85, onPress: () => hidden.set(!hidden.get()) }),
        view.node("Text", { text: resizeLabel, size: 14, width: 85, onPress: () => resize.set(!resize.get()) })),
      view.node("Text", { text: summary, size: 14 }))),
    view.node("Stack", { gap: 4, hidden }, ...Array.from({ length: 50 }, (_, row) =>
      view.node("Stack", { axis: "horizontal", gap, hidden: hiddenRows.at(row) },
        ...Array.from({ length: 10 }, (_, column) => {
          const index = row * 10 + column;
          return view.node("Text", {
            text: labels.at(index), hidden: hiddenCells.at(index),
            size: 8, width: 30, maxLines: 1, tabularNumbers: true,
          });
        })))),
  );
});
