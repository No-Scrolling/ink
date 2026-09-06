import { createElement, useRef, useState, type ReactNode } from "react";

type ListSizing = { itemHeight: number; estimatedItemHeight?: never }
  | { itemHeight?: never; estimatedItemHeight: number };

export type ListProps<T> = ListSizing & {
  items: readonly T[];
  keyExtractor: (item: T, index: number) => string;
  renderItem: (item: T, index: number) => ReactNode;
  gap?: number;
  followEnd?: boolean;
  measurementKey?: string | number;
};

export function List<T>({ items, itemHeight, estimatedItemHeight, keyExtractor, renderItem, gap = 0, followEnd = false, measurementKey }: ListProps<T>) {
  const height = itemHeight ?? estimatedItemHeight;
  if (height === undefined || !Number.isFinite(height) || height <= 0 || !Number.isFinite(gap) || gap < 0) {
    throw new Error("List heights must be positive and finite; gap must be non-negative");
  }
  const previous = useRef<{
    measurementKey?: string | number;
    items: readonly T[]; keys: string[]; contentVersions: number[]; revision: number;
    records: Map<string, { item: T; version: number }>;
  } | null>(null);
  const [window, setWindow] = useState({ start: 0, end: 32 });
  const keys = items.map(keyExtractor);
  if (keys.some(key => typeof key !== "string") || new Set(keys).size !== keys.length) {
    throw new Error("List keys must be unique strings");
  }
  let start = Math.min(window.start, items.length);
  let end = Math.min(window.end, items.length);
  let data = previous.current;
  const old = data;
  const keysChanged = old && (keys.length !== old.keys.length || keys.some((key, i) => key !== old.keys[i]));
  if (!data || data.items !== items || data.measurementKey !== measurementKey || keysChanged) {
    const revision = (old?.revision ?? 0) + 1;
    const records = new Map(keys.map((key, index) => {
      const oldRecord = old?.records.get(key);
      return [key, { item: items[index], version: oldRecord && old?.measurementKey === measurementKey && Object.is(oldRecord.item, items[index]) ? oldRecord.version : revision }];
    }));
    const contentVersions = Array.from(records.values(), record => record.version);
    if (new TextEncoder().encode(JSON.stringify({ keys, contentVersions })).length > 128 * 1024) {
      throw new Error("List metadata exceeds 128 KiB; use shorter keys or a bounded data window");
    }
    if (old && old.keys[window.start] !== undefined) {
      const mapped = keys.indexOf(old.keys[window.start]);
      start = mapped < 0 ? Math.min(window.start, Math.max(0, items.length - 1)) : mapped;
      end = Math.min(start + window.end - window.start, items.length);
      if (start !== window.start || end !== window.end) setWindow({ start, end });
    }
    data = { items, keys, contentVersions, revision, records, measurementKey };
    previous.current = data;
  }
  const { revision, contentVersions } = data;
  const children: ReactNode[] = [];
  for (let index = start; index < end; index++) {
    children.push(createElement("Stack", { key: keys[index], axis: "vertical", align: "stretch" }, renderItem(items[index], index)));
  }
  const props = {
    keys: data.keys,
    contentVersions,
    revision,
    itemHeight,
    estimatedItemHeight,
    gap,
    followEnd,
    start,
    onWindow(start: number, end: number, eventRevision: number) {
      if (eventRevision !== previous.current?.revision) return;
      setWindow(previous => previous.start === start && previous.end === end ? previous : { start, end });
    },
  };
  return createElement("List", props, children);
}
