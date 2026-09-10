import { createElement } from "./react";
import { Button, Stack, Text } from "./index";
import { keyboardArrowDown, keyboardArrowUp } from "./icons";
import { List } from "./list";

export type ReorderListProps<T> = {
  items: readonly T[];
  keyExtractor: (item: T) => string;
  getLabel: (item: T) => string;
  onChange: (items: T[]) => void;
  multiline?: boolean;
};

export function ReorderList<T>({ items, keyExtractor, getLabel, onChange, multiline = false }: ReorderListProps<T>) {
  function move(index: number, direction: -1 | 1) {
    const target = index + direction;
    if (target < 0 || target >= items.length) return;
    const next = [...items];
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  }

  return createElement(List<T>, {
    items,
    keyExtractor,
    measurementKey: multiline ? "multiline" : "single-line",
    renderItem: (item, index) => createElement(Stack, {
      axis: "horizontal", align: "center", justify: "space-between",
    },
    createElement(Text, { maxLines: multiline ? undefined : 1 }, getLabel(item)),
    createElement(Stack, { axis: "horizontal", align: "center", gap: 4 },
      createElement(Button, { icon: keyboardArrowDown, disabled: index === items.length - 1, onPress: () => move(index, 1) }),
      createElement(Button, { icon: keyboardArrowUp, disabled: index === 0, onPress: () => move(index, -1) }),
    )),
  });
}
