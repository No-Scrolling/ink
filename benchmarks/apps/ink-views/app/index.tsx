import { expression, nativeView } from "ink/view";
import { refresh } from "ink/icons";

export default nativeView(view => {
  const count = view.value(100);
  const tick = view.value(0);
  const reverse = view.value(false);
  const hidden = view.value(false);
  const resize = view.value(false);
  const labels = Array.from({ length: 500 }, (_, index) => {
    const id = view.compute(expression.choose(reverse, expression.subtract(count, index + 1), index));
    const prefix = view.compute(expression.concat(expression.pad(id, 3), ":"));
    return view.compute(expression.concat(prefix, expression.remainder(expression.add(id, tick), 100)));
  });
  const gap = view.compute(expression.choose(resize, expression.remainder(tick, 2), 0));
  const summary = view.compute(expression.concat(count, " cells · ", tick, " updates"));
  const hiddenRows = Array.from({ length: 50 }, (_, row) => view.compute(expression.not(expression.less(row * 10, count))));
  const hiddenCells = Array.from({ length: 500 }, (_, id) => view.compute(expression.not(expression.less(id, count))));
  const hiddenLabel = view.compute(expression.choose(hidden, "Show", "Hide"));
  const resizeLabel = view.compute(expression.choose(resize, "Resize on", "Resize off"));
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
      view.node("Stack", { axis: "horizontal", gap, hidden: hiddenRows[row] },
        ...Array.from({ length: 10 }, (_, column) => {
          const index = row * 10 + column;
          return view.node("Text", {
            text: labels[index], hidden: hiddenCells[index],
            size: 8, width: 30, maxLines: 1, tabularNumbers: true,
          });
        })))),
  );
});
