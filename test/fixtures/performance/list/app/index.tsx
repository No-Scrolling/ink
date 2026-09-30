import { useEffect, useState } from "react";
import { List, Screen, Text } from "ink";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;
type Item = { id: string; label: string };

export default function App() {
  const [items, setItems] = useState<readonly Item[]>([]);
  useEffect(() => {
    const stop = onNativeMessage("benchmark", (message) => {
      if (message.command === "populate")
        setItems(
          Array.from({ length: Number(message.count) }, (_, index) => ({
            id: String(index),
            label: `Row ${String(index).padStart(5, "0")}: 0`,
          })),
        );
      if (message.command === "edit")
        setItems((previous) => {
          const next = [...previous];
          next[0] = { id: "0", label: `Row 00000: ${Number(message.value)}` };
          return next;
        });
    });
    __inkPost(JSON.stringify({ type: "ready" }));
    return stop;
  }, []);
  return (
    <Screen title="List">
      <List
        items={items}
        gap={8}
        keyExtractor={(item) => item.id}
        renderItem={(item) => <Text>{item.label}</Text>}
      />
    </Screen>
  );
}
