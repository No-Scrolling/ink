import { createElement, Fragment, useContext, useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "./react";

import { useAction } from "./action";
import { Button, Stack, Text } from "./index";
import { CompiledList, nativeListPlan } from "./native-list";
import { ScrollToEndContext } from "./scroll";

function ListLoadError({ message, onRetry }: { message: string; onRetry: () => void }) {
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
  initialPosition?: "start" | "end";
};

type PaginationProps = Pick<ListProps<unknown>, "onLoadMore" | "hasMore" | "onLoadOlder" | "hasOlder">;
export type ListPagination = ReturnType<typeof useListPagination>;

function useListPage(load: (() => Promise<void>) | undefined, available: boolean) {
  const accepted = useRef<string | undefined>(undefined);
  const action = useAction((edge?: string) => {
    // useAction may decline a pending request; record acceptance only when it starts.
    if (edge !== undefined) accepted.current = edge;
    return load?.();
  });
  return {
    available: available && action.status !== "error",
    enabled: !!load,
    pending: action.status === "pending",
    error: action.status === "error" ? action.error : undefined,
    retry: () => action.run(),
    request(edge: string) {
      if (available && load && action.status !== "pending" && action.status !== "error" && accepted.current !== edge) action.run(edge);
    },
  };
}

function useListPagination({ onLoadMore, hasMore = false, onLoadOlder, hasOlder = false }: PaginationProps) {
  const more = useListPage(onLoadMore, hasMore);
  const older = useListPage(onLoadOlder, hasOlder);
  return {
    more,
    older,
    wrap(list: ReactNode) {
      return createElement(Fragment, null,
        older.error && createElement(ListLoadError, { message: older.error.message, onRetry: older.retry }),
        list,
        more.error && createElement(ListLoadError, { message: more.error.message, onRetry: more.retry }),
      );
    },
  };
}

export function useListPaginationEvents(pagination: ListPagination, paused: boolean, count: number, first?: string, last?: string) {
  const [startRequest, setStartRequest] = useState<string>();
  const [endRequest, setEndRequest] = useState<string>();
  const start = JSON.stringify([count, first ?? null]);
  const end = JSON.stringify([count, last ?? null]);
  useEffect(() => {
    if (!paused && startRequest === start) pagination.older.request(startRequest);
    if (!paused && endRequest === end) pagination.more.request(endRequest);
  }, [pagination, paused, startRequest, endRequest, start, end]);
  return {
    hasMore: !paused && pagination.more.available && !pagination.more.pending,
    hasOlder: !paused && pagination.older.available && !pagination.older.pending,
    moreAvailable: pagination.more.available && pagination.more.enabled,
    // Retain notifications even when the current commit cannot start loading yet.
    onEndReached: setEndRequest,
    onStartReached: setStartRequest,
  };
}

export function List<T>(props: ListProps<T>) {
  const pagination = useListPagination(props);
  const requestEnd = useContext(ScrollToEndContext);
  const initialised = useRef(false);
  useLayoutEffect(() => {
    if (initialised.current || props.initialPosition === "end" && !props.items.length) return;
    initialised.current = true;
    if (props.initialPosition === "end") requestEnd?.();
  }, [props.initialPosition, props.items.length, requestEnd]);
  const plan = nativeListPlan(props.renderItem);
  return pagination.wrap(plan
    ? createElement(CompiledList<T>, { ...props, pagination, plan, key: plan.template })
    : createElement(ReactList<T>, { ...props, pagination }));
}

export function ReactList<T>({ items, keyExtractor, renderItem, gap = 47, followEnd = false, measurementKey, initialPosition = "start", pagination }: ListProps<T> & { pagination: ListPagination }) {
  if (!Number.isFinite(gap) || gap < 0) {
    throw new Error("List gap must be finite and non-negative");
  }
  const initialWindow = useRef(initialPosition === "end");
  const previous = useRef<{
    measurementKey?: string | number;
    keyExtractor: ListProps<T>["keyExtractor"];
    items: readonly T[]; keys: string[]; contentVersions: number[]; revision: number;
    records: Map<string, { item: T; version: number }>;
  } | null>(null);
  const [window, setWindow] = useState({ start: initialPosition === "end" ? Math.max(0, items.length - 32) : 0, end: initialPosition === "end" ? items.length : 32, revision: 0 });
  let start = Math.min(window.start, items.length);
  let end = Math.min(window.end, items.length);
  if (initialWindow.current && items.length) {
    start = Math.max(0, items.length - 32);
    end = items.length;
  }
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
    if (initialWindow.current && items.length) {
      if (start !== window.start || end !== window.end) setWindow({ start, end, revision });
    } else if (old && window.revision !== revision && old.keys[window.start] !== undefined) {
      const mapped = keys.indexOf(old.keys[window.start]);
      start = mapped < 0 ? Math.min(window.start, Math.max(0, items.length - 1)) : mapped;
      end = Math.min(start + window.end - window.start, items.length);
      if (start !== window.start || end !== window.end) setWindow({ start, end, revision });
    }
    data = { items, keys, keyExtractor, contentVersions, revision, records, measurementKey };
  }
  if (data.keyExtractor !== keyExtractor) data = { ...data, keyExtractor };
  useLayoutEffect(() => {
    previous.current = data;
    if (items.length) initialWindow.current = false;
  });
  const { revision, contentVersions } = data;
  const paginationProps = useListPaginationEvents(pagination, window.revision !== revision, keys.length, keys[0], keys.at(-1));
  const children: ReactNode[] = [];
  for (let index = start; index < end; index++) {
    children.push(createElement("Stack", { key: keys[index], axis: "vertical", align: "stretch" }, renderItem(items[index], index)));
  }
  const props = {
    ...paginationProps,
    keys: data.keys,
    contentVersions,
    revision,
    gap,
    followEnd,
    start,
    onWindow(start: number, end: number, eventRevision: number) {
      if (eventRevision !== revision) return;
      setWindow(previous => previous.start === start && previous.end === end && previous.revision === eventRevision ? previous : { start, end, revision: eventRevision });
    },
  };
  return createElement("List", props, children);
}
