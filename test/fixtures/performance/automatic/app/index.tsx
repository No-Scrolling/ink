import { useEffect, useEffectEvent, useState } from "react";
import { Button, List, Row, Screen, Stack, Text, TextInput } from "ink";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (message: string) => void;
type Item = { id: string; title: string; subtitle: string };
const itemsFor = (count: number, generation: number): Item[] =>
  Array.from({ length: count }, (_, index) => ({
    id: String(index),
    title: `Item ${String(index).padStart(5, "0")}`,
    subtitle: `Generation ${generation}`,
  }));

export default function App() {
  const [items, setItems] = useState(() => itemsFor(1000, 0));
  const [draft, setDraft] = useState("");
  const [selected, setSelected] = useState("none");
  const [generation, setGeneration] = useState(0);
  const [count, setCount] = useState(1000);
  const [shown, setShown] = useState(true);
  const replace = () => {
    const next = generation + 1;
    setGeneration(next);
    setItems(itemsFor(count, next));
  };
  const receive = useEffectEvent((message: Record<string, unknown>) => {
    if (message.command === "populate") {
      const size = Number(message.count);
      setCount(size);
      setGeneration(0);
      setItems(itemsFor(size, 0));
    }
    if (message.command === "replace") {
      const next = Number(message.generation);
      setGeneration(next);
      setItems(itemsFor(count, next));
    }
    if (message.command === "edit") {
      const next = Number(message.generation);
      setGeneration(next);
      setItems((previous) => {
        const result = previous.slice();
        const index = result.findIndex((item) => item.id === "0");
        result[index] = { ...result[index], subtitle: `Generation ${next}` };
        return result;
      });
    }
    if (message.command === "reorder") setItems((previous) => [...previous].reverse());
    if (message.command === "shown") setShown(message.value === true);
    if (message.command === "report")
      __inkPost(
        JSON.stringify({
          type: "observed",
          draft,
          generation,
          selected,
          shown,
          count: items.length,
          first: items[0]?.id,
          last: items.at(-1)?.id,
        }),
      );
  });
  useEffect(() => {
    const stop = onNativeMessage("benchmark", receive);
    __inkPost(JSON.stringify({ type: "ready" }));
    return stop;
  }, []);
  return (
    <Screen title={`Lists ${selected}`} wide>
      <Stack axis="horizontal" gap={16}>
        <Button onPress={replace}>Replace</Button>
        <Button onPress={() => setShown((value) => !value)}>Show</Button>
      </Stack>
      <TextInput value={draft} onChange={setDraft} autoFocus placeholder="Type here" />
      <Text width={700} maxLines={1}>
        Requested {generation}
      </Text>
      {shown && (
        <List
          items={items}
          keyExtractor={(item) => item.id}
          gap={8}
          renderItem={(item, index) => (
            <Row
              title={item.title}
              subtitle={item.subtitle}
              titleMaxLines={2}
              onPress={() => setSelected(`${item.id}:${index}:${item.subtitle}`)}
            />
          )}
        />
      )}
    </Screen>
  );
}
