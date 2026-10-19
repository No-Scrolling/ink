import { useEffect, useState } from "react";
import { List, Row, Screen, Stack, Text } from "ink";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;
type Item = { id: string; title: string; subtitle: string; image?: string };

const PureTrack = ({ item, onPress }: { item: Item; onPress: () => void }) => (
  <Stack axis="horizontal" align="start" gap={8} onPress={onPress}>
    <Text width={40} size={26} maxLines={1}>
      {item.id}
    </Text>
    <Row title={item.title} titleMaxLines={2} subtitle={item.subtitle} />
  </Stack>
);

function Track(props: { item: Item; onPress: () => void }) {
  return <PureTrack {...props} />;
}

export default function App() {
  const [items, setItems] = useState<readonly Item[]>([]);
  const [compatibility, setCompatibility] = useState(false);
  const [pure, setPure] = useState(false);
  const [selection, setSelection] = useState("none");
  useEffect(() => {
    const stop = onNativeMessage("benchmark", (message) => {
      if (message.command === "populate") {
        setItems(message.items as Item[]);
        setCompatibility(message.compatibility === true);
        setPure(message.pure === true);
      }
      if (message.command === "edit")
        setItems((previous) =>
          previous.map((item) =>
            item.id === "0" ? { ...item, subtitle: String(message.subtitle) } : item,
          ),
        );
      if (message.command === "prepend")
        setItems((previous) =>
          message.insert
            ? [message.item as Item, ...previous]
            : previous.filter((item) => item.id !== "new"),
        );
      if (message.command === "reorder") setItems((previous) => [...previous].reverse());
    });
    __inkPost(JSON.stringify({ type: "ready" }));
    return stop;
  }, []);
  return (
    <Screen title={`Rows ${selection}`} wide>
      {pure ? (
        <List items={items} gap={8} keyExtractor={(item) => item.id}
          renderItem={(item) => <PureTrack item={item} onPress={() => setSelection(item.id)} />} />
      ) : compatibility ? (
        <List
          items={items}
          gap={8}
          keyExtractor={(item) => item.id}
          renderItem={(item) => <Track item={item} onPress={() => setSelection(item.id)} />}
        />
      ) : (
        <List
          items={items}
          gap={8}
          keyExtractor={(item) => item.id}
          renderItem={(item) => (
            <Row
              title={item.title}
              titleMaxLines={2}
              subtitle={item.subtitle}
              image={item.image}
              onPress={() => setSelection(item.id)}
            />
          )}
        />
      )}
    </Screen>
  );
}
