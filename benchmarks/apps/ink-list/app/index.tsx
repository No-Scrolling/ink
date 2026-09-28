import { useState } from "react";
import { ConversationScreen, ReorderList, List, Row, Screen, Stack, Text } from "ink";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (message: string) => void;
onNativeMessage("list-barrier", () => queueMicrotask(() => queueMicrotask(() => {
  __inkPost(JSON.stringify({ type: "list-settled" }));
})));

type Item = { id: string; label: string };
const initial = (): Item[] => Array.from({ length: 1000 }, (_, i) => ({ id: `row-${i}`, label: `Row ${i}` }));
function StatefulRow({ item, select }: { item: Item; select: (key: string) => void }) {
  const [count, setCount] = useState(0);
  return <Stack gap={4}>
    <Text size={22} onPress={() => { setCount(count + 1); select(`${item.id}:${count + 1}`); }}>{item.label}</Text>
    <Text size={12}>日本語 👩🏽‍🚀</Text>
  </Stack>;
}
function InlineLabel({ label }: { label: string }) {
  const [value] = useState(label);
  return value;
}
export default function Lists() {
  const [items, setItems] = useState(initial);
  const [mode, setMode] = useState("Native");
  const [selected, select] = useState("No selection");
  const [capture, setCapture] = useState(0);
  const [edit, setEdit] = useState(0);
  const [tick, setTick] = useState(0);
  const [draft, setDraft] = useState("");
  if (mode === "Reorder") return <Screen title="Reorder checks" header={<Text onPress={() => setMode("Messages")}>Messages</Text>}>
    <ReorderList items={items} keyExtractor={item => item.id} getLabel={item => item.label} onChange={setItems} />
  </Screen>;
  if (mode === "Messages") return <ConversationScreen title="Messages" messages={items.map(item => ({
    id: item.id, text: `${item.label} 日本語 👩🏽‍🚀`, timestamp: 1700000000000, outgoing: false,
  }))} draft={draft} onDraftChange={setDraft} onSend={() => setDraft("")} />;
  return <Screen title="Default lists" header={<Stack gap={12}>
      <Stack axis="horizontal" gap={10}>
        <Text size={16} width={70} onPress={() => setMode("Native")}>Native</Text>
        <Text size={16} width={70} onPress={() => setMode("React")}>React</Text>
        <Text size={16} width={70} onPress={() => setMode("Row")}>Row</Text>
        <Text size={16} width={70} onPress={() => setMode("Rich")}>Rich</Text>
      </Stack>
      <Stack axis="horizontal" gap={10}>
        <Text size={16} width={90} onPress={() => { const id = `added-${tick}`; setTick(tick+1); setItems([{ id, label: id }, ...items]); }}>Prepend</Text>
        <Text size={16} width={90} onPress={() => setItems([...items].reverse())}>Reverse</Text>
        <Text size={16} width={90} onPress={() => { setItems(initial()); select("No selection"); }}>Reset</Text>
      </Stack>
      <Stack axis="horizontal" gap={10}>
        <Text size={16} width={90} onPress={() => setCapture(capture + 1)}>Unrelated</Text>
        <Text size={16} width={90} onPress={() => { setEdit(edit + 1); setItems(items.map((item, i) => i === 0 ? { ...item, label: `Edited ${edit + 1}` } : item)); }}>Edit</Text>
        <Text size={16} width={90} onPress={() => setItems(items.slice(1))}>Delete</Text>
      </Stack>
      <Text size={14} onPress={() => setMode("Reorder")}>Built-ins</Text>
      <Text size={14}>Capture {capture}</Text>
      <Text size={14}>{mode}: {items.length} rows</Text>
      <Text size={14}>{selected}</Text>
    </Stack>}>
    {mode === "Rich" ? <List key="rich" items={items.map(item => ({ ...item, content: <InlineLabel label={item.label} /> }))}
      gap={12} keyExtractor={item => item.id}
      renderItem={item => <Stack gap={4}>
        <Text size={22} onPress={() => select(capture ? `${item.id}@${capture}` : item.id)}>{item.content}</Text>
        <Text size={12}>日本語 👩🏽‍🚀</Text>
      </Stack>} />
      : mode === "React" ? <List key="react" items={items} gap={12} keyExtractor={item => item.id}
      renderItem={item => <StatefulRow item={item} select={select} />} />
      : mode === "Row" ? <List key="row" items={items} gap={12} keyExtractor={item => item.id}
        renderItem={item => <Row title={item.label} subtitle="日本語 👩🏽‍🚀" onPress={() => select(capture ? `${item.id}@${capture}` : item.id)} />} />
      : <List key="native" items={items} gap={12} keyExtractor={item => item.id}
        renderItem={item => <Stack gap={4}>
          <Text size={22} onPress={() => select(capture ? `${item.id}@${capture}` : item.id)}>{item.label}</Text>
          <Text size={12}>日本語 👩🏽‍🚀</Text>
        </Stack>} />}
  </Screen>;
}
