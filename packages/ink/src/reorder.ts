import { createElement } from "./react";
import { Button, Stack, Text } from "./index";
import { keyboardArrowDown, keyboardArrowUp } from "./icons";
import { List } from "./list";
import { nativeListRow, nativeListTemplate } from "./native-list";

const template = nativeListTemplate((node, field) => node("Stack", { axis: "horizontal", align: "center", justify: "space-between" }, [
  node("Text", { text: field("label"), maxLines: field("maxLines") }),
  node("Stack", { axis: "horizontal", align: "center", gap: 4 }, [
    node("Button", { icon: keyboardArrowDown, disabled: field("last"), onPress: field("down") }),
    node("Button", { icon: keyboardArrowUp, disabled: field("first"), onPress: field("up") }),
  ]),
]));

export type ReorderListProps<T> = {
  items: readonly T[];
  keyExtractor: (item: T) => string;
  getLabel: (item: T) => string;
  onChange: (items: T[]) => void;
  multiline?: boolean;
  gap?: number;
};

export function ReorderList<T>({ items, keyExtractor, getLabel, onChange, multiline = false, gap }: ReorderListProps<T>) {
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
    gap,
    measurementKey: multiline ? "multiline" : "single-line",
    renderItem: nativeListRow((item: T, index: number) => createElement(Stack, {
      axis: "horizontal", align: "center", justify: "space-between",
    },
    createElement(Text, { maxLines: multiline ? undefined : 1 }, getLabel(item)),
    createElement(Stack, { axis: "horizontal", align: "center", gap: 4 },
      createElement(Button, { icon: keyboardArrowDown, disabled: index === items.length - 1, onPress: () => move(index, 1) }),
      createElement(Button, { icon: keyboardArrowUp, disabled: index === 0, onPress: () => move(index, -1) }),
    )), (item, index) => [{ label: getLabel(item), maxLines: multiline ? undefined : 1,
      first: index === 0, last: index === items.length - 1,
      down: () => move(index, 1), up: () => move(index, -1) }], template,
      (item, index) => [getLabel(item), multiline, index, items.length]),
  });
}
