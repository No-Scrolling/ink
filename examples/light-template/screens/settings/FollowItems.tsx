import { useState } from "react";
import { Button, List, Screen, Text } from "ink";

export default function FollowItems() {
  const [items, setItems] = useState([0]);
  return (
    <Screen title="Follow new items" header={<>
      <Button onPress={() => setItems(current => [...current, current.length])}>Add new item</Button>
      <Button onPress={() => setItems([])}>Clear items</Button>
    </>}>
      <List items={items} gap={47} followEnd
        keyExtractor={item => String(item)}
        renderItem={item => <Text>Item {item}</Text>} />
    </Screen>
  );
}
