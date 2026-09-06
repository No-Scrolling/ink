import { useState } from "react";
import { Button, Screen, Text, Toggle } from "ink";

export default function DynamicUI() {
  const [showItems, setShowItems] = useState(true);
  const [items, setItems] = useState<number[]>([]);

  return (
    <Screen title="Dynamic UI">
      <Toggle label="Show Items" value={showItems} onChange={setShowItems} />
      {showItems && <>
        <Button onPress={() => setItems(current => [...current, current.length + 1])}>Add Items</Button>
        <Button onPress={() => setItems([])}>Clear Items</Button>
        {items.map(item => <Text key={item}>Item {item}</Text>)}
      </>}
    </Screen>
  );
}
