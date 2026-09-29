import { expression, nativeView } from "ink/view";

export default nativeView(view => {
  const count = view.value(0);
  const label = view.compute(expression.concat("Count: ", count));
  return view.node("Screen", { title: "Counter", centered: true },
    view.node("Stack", { gap: 16, align: "center" },
      view.node("Text", { size: 40, text: label }),
      view.node("Button", { onPress: () => count.set(count.get() + 1) }, view.node("Text", { text: "Increase" }))),
  );
});
