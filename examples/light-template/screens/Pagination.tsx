import { useState } from "react";
import { List, Screen, Text } from "ink";

const pageSize = 20;
const itemCount = 100;

export default function Pagination() {
  const [items, setItems] = useState(() => Array.from({ length: pageSize }, (_, index) => index));
  async function loadMore() {
    await new Promise<void>(resolve => setTimeout(() => resolve(), 300));
    setItems(current => [...current, ...Array.from({ length: Math.min(pageSize, itemCount - current.length) }, (_, index) => current.length + index)]);
  }
  return <Screen title="Pagination">
    <List items={items} onLoadMore={loadMore} hasMore={items.length < itemCount}
      keyExtractor={item => String(item)} renderItem={item => <Text>Item {item}</Text>} />
  </Screen>;
}
