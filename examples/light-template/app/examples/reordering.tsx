import { useState } from "react";
import { Button, Screen, Stack, Text } from "ink";
import { keyboardArrowDown, keyboardArrowUp } from "ink/icons";

export default function Reordering() {
  const [items, setItems] = useState(() => Array.from({ length: 8 }, (_, index) => index));

  function move(item: number, direction: -1 | 1) {
    setItems(current => {
      const index = current.indexOf(item);
      const target = index + direction;
      if (index < 0 || target < 0 || target >= current.length) return current;
      const next = [...current];
      [next[index], next[target]] = [next[target], next[index]];
      return next;
    });
  }

  return (
    <Screen title="Reordering">
      {items.map((item, index) => (
        <Stack key={item} axis="horizontal" align="center" justify="space-between">
          <Text maxLines={1}>Item {item}</Text>
          <Stack axis="horizontal" align="center" gap={4}>
            <Button icon={keyboardArrowDown} disabled={index === items.length - 1}
              onPress={() => move(item, 1)} />
            <Button icon={keyboardArrowUp} disabled={index === 0}
              onPress={() => move(item, -1)} />
          </Stack>
        </Stack>
      ))}
    </Screen>
  );
}
