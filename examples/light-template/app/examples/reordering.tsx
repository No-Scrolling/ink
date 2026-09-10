import { useState } from "react";
import { ReorderList, Screen } from "ink";

export default function Reordering() {
  const [items, setItems] = useState(() => Array.from({ length: 8 }, (_, index) => index));
  return <Screen title="Reordering">
    <ReorderList items={items} keyExtractor={String} getLabel={item => `Item ${item}`} onChange={setItems} />
  </Screen>;
}
