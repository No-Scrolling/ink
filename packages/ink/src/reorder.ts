import { createElement } from "./react";
import { keyboardArrowDown, keyboardArrowUp } from "./icons";
import { List } from "./list";
import { fixedListRow } from "./native-list";

type ReorderFields = {
  label: string; maxLines?: number; first: boolean; last: boolean;
  down: () => void; up: () => void;
};

const reorderRow = /* @__PURE__ */ fixedListRow<ReorderFields>((node, field) => node("Stack", { axis: "horizontal", align: "center", justify: "space-between", gap: 12 }, [
  node("Text", { text: field(values => values.label), maxLines: field(values => values.maxLines) }),
  node("Stack", { axis: "horizontal", align: "center", gap: 4 }, [
    node("Button", { icon: keyboardArrowDown, disabled: field(values => values.last), onPress: field(values => values.down) }),
    node("Button", { icon: keyboardArrowUp, disabled: field(values => values.first), onPress: field(values => values.up) }),
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
    renderItem: reorderRow((item: T, index: number) => ({ label: getLabel(item), maxLines: multiline ? undefined : 1,
      first: index === 0, last: index === items.length - 1,
      down: () => move(index, 1), up: () => move(index, -1) })),
  });
}
