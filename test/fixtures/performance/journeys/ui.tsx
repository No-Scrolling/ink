import { List, Stack, Text } from "ink";
import type { Item } from "./data";

export function JourneyRows({ items }: { items: readonly Item[] }) {
  // CompiledList suppresses hasMore during preparation. The host observes its restoration
  // alongside the projected data as a production end-of-preparation marker. No loader runs.
  return <List items={items} gap={8} hasMore keyExtractor={item => item.id}
    renderItem={item => <Stack gap={4}>
      <Text width={950} size={item.titleSize} maxLines={2}>{item.title}</Text>
      <Text width={950} size={22} maxLines={1}>{item.subtitle}</Text>
    </Stack>} />;
}
