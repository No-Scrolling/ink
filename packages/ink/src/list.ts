import { createElement, useState, type ReactNode } from "react";

export interface ListProps<T> {
  items: readonly T[];
  itemHeight: number;
  keyExtractor: (item: T, index: number) => string;
  renderItem: (item: T, index: number) => ReactNode;
}

export function List<T>({ items, itemHeight, keyExtractor, renderItem }: ListProps<T>) {
  if (!Number.isFinite(itemHeight) || itemHeight <= 0) {
    throw new Error("List itemHeight must be positive and finite");
  }
  const [window, setWindow] = useState({ start: 0, end: 32 });
  const start = Math.min(window.start, items.length);
  const end = Math.min(window.end, items.length);
  const keys = new Set<string>();
  const children: ReactNode[] = [];
  for (let index = start; index < end; index++) {
    const item = items[index];
    const key = keyExtractor(item, index);
    if (keys.has(key)) throw new Error(`Duplicate List key: ${key}`);
    keys.add(key);
    children.push(createElement("Stack", { key, axis: "vertical", align: "stretch" }, renderItem(item, index)));
  }
  const props = {
    count: items.length,
    itemHeight,
    start,
    onWindow(start: number, end: number) {
      setWindow(previous => previous.start === start && previous.end === end ? previous : { start, end });
    },
  };
  return createElement("List", props, children);
}
