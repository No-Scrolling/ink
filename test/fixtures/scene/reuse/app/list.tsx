import { List, Screen, Text, navigate } from "ink";

const items = Array.from({ length: 2000 }, (_, index) => ({
  id: String(index),
  label: `Item ${index}`,
}));

export default function Rows() {
  return (
    <Screen title="Rows">
      <List
        items={items}
        gap={8}
        keyExtractor={(item) => item.id}
        renderItem={(item) => <Text onPress={() => navigate("/detail")}>{item.label}</Text>}
      />
    </Screen>
  );
}
