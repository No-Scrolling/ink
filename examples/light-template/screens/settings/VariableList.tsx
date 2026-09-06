import { useState } from "react";
import { Button, List, Screen, Stack, Text, Toggle } from "ink";

const makeItem = (id: number) => ({ id, text: `Item ${id}. ${"This row wraps naturally as its content grows. ".repeat(Math.abs(id) % 4 + 1)}` });

export default function VariableList() {
  const [items, setItems] = useState(() => Array.from({ length: 200 }, (_, index) => makeItem(index + 1)));
  const [follow, setFollow] = useState(false);
  return <Screen title="Variable-height List">
    <Toggle label="Follow new items" value={follow} onChange={setFollow} />
    <Button onPress={() => setItems(current => [makeItem(current[0].id - 1), ...current])}>Add earlier item</Button>
    <Button onPress={() => setItems(current => [...current, makeItem(current[current.length - 1].id + 1)])}>Add new item</Button>
    <List items={items} estimatedItemHeight={140} gap={47} followEnd={follow}
      keyExtractor={item => String(item.id)}
      renderItem={item => <Stack gap={12}>
        <Text size={18}>{item.text}</Text>
        <Button onPress={() => setItems(current => current.map(row => row.id === item.id
          ? { ...row, text: `${row.text} This is some more text added to this item.` } : row))}>Expand item</Button>
      </Stack>} />
  </Screen>;
}
