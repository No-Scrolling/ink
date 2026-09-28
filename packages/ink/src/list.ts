import { createElement, Fragment, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "./react";

import { useAction } from "./action";
import { Button, Stack, Text } from "./index";
import { CompiledList, nativeListPlan } from "./native-list";

const OLDER_PAGE_AHEAD_ITEMS = 8;

export function ListLoadError({ message, onRetry }: { message: string; onRetry: () => void }) {
  return createElement(Stack, { gap: 24 },
    createElement(Text, { size: 18 }, message),
    createElement(Button, { onPress: onRetry }, "Try again"),
  );
}

export type ListProps<T> = {
  items: readonly T[];
  keyExtractor: (item: T, index: number) => string;
  renderItem: (item: T, index: number) => ReactNode;
  gap?: number;
  followEnd?: boolean;
  measurementKey?: string | number;
  onLoadMore?: () => Promise<void>;
  hasMore?: boolean;
  onLoadOlder?: () => Promise<void>;
  hasOlder?: boolean;
  initialEnd?: boolean;
};

export function List<T>(props: ListProps<T>) {
  const plan = nativeListPlan(props.renderItem);
  return plan ? createElement(CompiledList<T>, { ...props, plan, key: plan.template }) : createElement(ReactList<T>, props);
}

export function ReactList<T>({ items, keyExtractor, renderItem, gap = 47, followEnd = false, measurementKey, onLoadMore, hasMore = true, onLoadOlder, hasOlder = false, initialEnd = false }: ListProps<T>) {
  if (!Number.isFinite(gap) || gap < 0) {
    throw new Error("List gap must be finite and non-negative");
  }
  const previous = useRef<{
    measurementKey?: string | number;
    keyExtractor: ListProps<T>["keyExtractor"];
    items: readonly T[]; keys: string[]; contentVersions: number[]; revision: number;
    records: Map<string, { item: T; version: number }>;
  } | null>(null);
  const [window, setWindow] = useState({ start: initialEnd ? Math.max(0, items.length - 32) : 0, end: initialEnd ? items.length : 32, revision: 0, nearEnd: false });
  const requested = useRef<string | null>(null);
  const load = useAction(() => onLoadMore?.());
  const olderRequested = useRef<string | null>(null);
  const older = useAction(() => onLoadOlder?.());
  let start = Math.min(window.start, items.length);
  let end = Math.min(window.end, items.length);
  let data = previous.current;
  const old = data;
  const keys = old && old.items === items && old.keyExtractor === keyExtractor ? old.keys : items.map(keyExtractor);
  if (keys !== old?.keys && (keys.some(key => typeof key !== "string") || new Set(keys).size !== keys.length)) {
    throw new Error("List keys must be unique strings");
  }
  const keysChanged = old && keys !== old.keys && (keys.length !== old.keys.length || keys.some((key, i) => key !== old.keys[i]));
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
    if (old && window.revision !== revision && old.keys[window.start] !== undefined) {
      const mapped = keys.indexOf(old.keys[window.start]);
      start = mapped < 0 ? Math.min(window.start, Math.max(0, items.length - 1)) : mapped;
      end = Math.min(start + window.end - window.start, items.length);
      if (start !== window.start || end !== window.end) setWindow({ start, end, revision, nearEnd: false });
    }
    data = { items, keys, keyExtractor, contentVersions, revision, records, measurementKey };
  }
  if (data.keyExtractor !== keyExtractor) data = { ...data, keyExtractor };
  useLayoutEffect(() => { previous.current = data; });
  const { revision, contentVersions } = data;
  const boundary = JSON.stringify([keys.length, keys.at(-1)]);
  useEffect(() => {
    if (!onLoadMore || !hasMore || load.status === "pending" || load.status === "error"
      || window.revision !== revision || !window.nearEnd || requested.current === boundary) return;
    requested.current = boundary;
    load.run();
  }, [onLoadMore, hasMore, load.status, load.run, window, revision, boundary]);
  const firstBoundary = JSON.stringify([keys.length, keys[0]]);
  useEffect(() => {
    if (!onLoadOlder || !hasOlder || older.status === "pending" || older.status === "error"
      || window.revision !== revision || window.start > OLDER_PAGE_AHEAD_ITEMS || olderRequested.current === firstBoundary) return;
    olderRequested.current = firstBoundary;
    older.run();
  }, [onLoadOlder, hasOlder, older.status, older.run, window, revision, firstBoundary]);
  const children: ReactNode[] = [];
  for (let index = start; index < end; index++) {
    children.push(createElement("Stack", { key: keys[index], axis: "vertical", align: "stretch" }, renderItem(items[index], index)));
  }
  const props = {
    keys: data.keys,
    contentVersions,
    revision,
    gap,
    followEnd,
    start,
    onWindow(start: number, end: number, eventRevision: number, nearEnd: boolean) {
      if (eventRevision !== revision) return;
      setWindow(previous => previous.start === start && previous.end === end && previous.revision === eventRevision && previous.nearEnd === nearEnd ? previous : { start, end, revision: eventRevision, nearEnd });
    },
  };
  const list = createElement("List", props, children);
  return createElement(Fragment, null,
    older.status === "error" && createElement(ListLoadError, { message: older.error.message, onRetry: older.run }),
    list,
    load.status === "error" && createElement(ListLoadError, { message: load.error.message, onRetry: load.run }),
  );
}
