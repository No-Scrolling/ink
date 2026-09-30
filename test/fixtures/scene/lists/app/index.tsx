import { useEffect, useState } from "react";
import { List, Screen, Text } from "ink";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;
type Item = { id: string; label: string };
const rows = Array.from({ length: 2000 }, (_, index) => ({
  id: String(index),
  label: `Item ${index}`,
}));
export default function App() {
  const [items, setItems] = useState<readonly Item[]>(rows);
  const [prefix, setPrefix] = useState("A");
  useEffect(
    () =>
      onNativeMessage("test", (message) => {
        if (message.command === "prefix") setPrefix(String(message.value));
        if (message.command === "prepend")
          setItems((previous) => [{ id: "new", label: "New item" }, ...previous]);
        if (message.command === "reverse") setItems((previous) => [...previous].reverse());
        if (message.command === "remove")
          setItems((previous) => previous.filter((item) => item.id !== message.key));
        if (message.command === "empty") setItems([]);
        if (message.command === "proxy")
          setItems([
            new Proxy(
              { id: "proxy", label: "Proxy item" },
              {
                get(target, key, receiver) {
                  return Reflect.get(target, key, receiver);
                },
              },
            ),
          ]);
      }),
    [],
  );
  return (
    <Screen title="List fixture">
      <List
        items={items}
        gap={8}
        keyExtractor={(item) => item.id}
        renderItem={(item, index) => (
          <Text
            onPress={() =>
              __inkPost(JSON.stringify({ type: "selected", key: item.id, index, prefix }))
            }
          >
            {prefix}: {item.label}
          </Text>
        )}
      />
    </Screen>
  );
}
