import { Screen, Stack, Text } from "ink";

const items = Array.from({ length: 1_000 }, (_, index) => `Item ${index + 1}`);

export default function Scroll() {
  return (
    <Screen title="Items">
      <Stack gap={24}>
        {items.map((item) => <Text key={item}>{item}</Text>)}
      </Stack>
    </Screen>
  );
}
