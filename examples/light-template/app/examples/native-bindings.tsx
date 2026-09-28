import { navigate } from "ink";
import { nativeView } from "ink/view";

export default nativeView(view => {
  const count = view.value(0);
  const playing = view.value(false);
  const position = view.value(0);
  const text = view.value("");
  const eventCount = view.value(0);
  const label = view.derive([count], () => `Count: ${count.get()}`);
  const playLabel = view.derive([playing], () => playing.get() ? "Pause" : "Play");
  const echoed = view.derive([text], () => `You typed: ${text.get()}`);
  let started = 0;
  return view.node("Screen", { title: "Native bindings" },
    view.node("Stack", { gap: 12 },
      view.node("Text", { size: 28, text: label }),
      view.node("Button", { onPress: () => count.set(count.get() + 1) }, view.node("Text", { text: "Increase" })),
      view.node("Text", { size: 18, text: "日本語 · ひらがな · カタカナ · 👩🏽‍🚀 🇯🇵" }),
      view.node("TextInput", {
        value: text, eventCount, placeholder: "Type here",
        onChange: (value, count) => {
          if (typeof value === "string" && typeof count === "number") { text.set(value); eventCount.set(count); }
        },
      }),
      view.node("Text", { size: 14, text: echoed }),
      view.node("PlayingProgress", { position, duration: 120, playing, showTimes: true,
        onSeek: seconds => { if (typeof seconds === "number") { position.set(seconds); started = performance.now(); } } }),
      view.node("Button", { onPress: () => {
        if (playing.get()) position.set(Math.min(120, position.get() + (performance.now() - started) / 1000));
        else started = performance.now();
        playing.set(!playing.get());
      } }, view.node("Text", { text: playLabel })),
      view.node("Button", { onPress: () => navigate("/examples/inputs") }, view.node("Text", { text: "Open React inputs" })),
    ),
  );
});
