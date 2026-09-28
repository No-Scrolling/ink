import { createElement, Fragment, useLayoutEffect, useMemo, useRef, type ComponentProps, type ReactNode } from "react";
import { ListLoadError, ReactList, type ListProps } from "./list";
import { useAction } from "./action";
import type { Row } from "./image";
import { navigate } from "./navigation";
import { rememberListPatch, type NativeListItem } from "./list-patch";

type Plan<T> = { project: (item: T, index: number) => unknown[]; template: string; dependencies?: (item: T, index: number) => unknown[] };
const needsReact = Symbol("list needs React");
const plans = new WeakMap<Function, Plan<unknown>>();

/** @internal Created by the Ink compiler; application List props stay unchanged. */
export function nativeListRow<T>(render: (item: T, index: number) => ReactNode, project: Plan<T>["project"], template: string, dependencies?: Plan<T>["dependencies"]) {
  plans.set(render, { project: project as Plan<unknown>["project"], template, dependencies: dependencies as Plan<unknown>["dependencies"] });
  return render;
}

/** @internal Match React's primitive text children without invoking components. */
export function listText(parts: unknown[]): string {
  return parts.map(value => {
    if (value == null || typeof value === "boolean") return "";
    if (Array.isArray(value)) return listText(value);
    if (!["string", "number", "bigint"].includes(typeof value)) throw needsReact;
    return String(value);
  }).join("");
}

/** @internal The same presentation fields as Ink's stateless Row component. */
export function listRowFields(props: ComponentProps<typeof Row>) {
  const { href } = props;
  if (href !== undefined && props.onPress) throw new Error("Row accepts either href or onPress");
  return { image: props.image, title: props.title, titleMaxLines: props.titleMaxLines,
    titleIcon: props.titleIcon, subtitle: props.subtitle, subtitleIcon: props.subtitleIcon,
    onLongPress: props.onLongPress,
    onPress: href === undefined ? props.onPress ?? (props.onLongPress ? () => {} : undefined) : () => navigate(href),
    hasImage: props.image !== undefined, hideImage: props.image === undefined,
    hasTitleIcon: props.titleIcon !== undefined, hideTitleIcon: props.titleIcon === undefined,
    hideSubtitle: props.subtitle === undefined && props.subtitleIcon === undefined,
    hideSubtitleIcon: props.subtitleIcon === undefined, hideSubtitleText: props.subtitle === undefined };
}

function nativeValue(value: unknown, previous?: unknown): unknown {
  if (typeof value === "function") return true;
  if (value == null) return null;
  if (Array.isArray(value)) {
    const before = Array.isArray(previous) ? previous : [];
    const next = value.map((item, index) => nativeValue(item, before[index]));
    return before.length === next.length && next.every((item, index) => Object.is(item, before[index])) ? before : next;
  }
  if (typeof value === "object") {
    const before = previous !== null && typeof previous === "object" && !Array.isArray(previous)
      ? previous as Record<string, unknown> : {};
    const next: Record<string, unknown> = {};
    let unchanged = true;
    for (const key of Object.keys(value)) {
      const item = nativeValue(Reflect.get(value, key), before[key]);
      next[key] = item;
      if (!Object.hasOwn(before, key) || !Object.is(item, before[key])) unchanged = false;
    }
    return unchanged && Object.keys(next).length === Object.keys(before).length ? before : next;
  }
  return value;
}

type NativeItem = NativeListItem;
type RowRecord<T> = { item: T; index: number; dependencies?: unknown[]; native: NativeItem };
type Records<T> = { data: NativeItem[]; rows: Map<string, RowRecord<T>>; template: string };

function sameDependencies(before: unknown[] | undefined, after: unknown[] | undefined): boolean {
  return before !== undefined && after !== undefined && before.length === after.length && after.every((value, index) => {
    // Objects may have been mutated in place: retain the full projection path.
    if (value !== null && typeof value === "object") return false;
    return Object.is(value, before[index]);
  });
}

export function nativeListPlan<T>(render: ListProps<T>["renderItem"]): Plan<T> | undefined {
  return plans.get(render) as Plan<T> | undefined;
}

export function CompiledList<T>({ plan, ...props }: ListProps<T> & { plan: Plan<T> }) {
  const { items, keyExtractor, gap = 47, followEnd, initialEnd, measurementKey, onLoadMore, onLoadOlder, hasMore = true, hasOlder = false } = props;
  if (!Number.isFinite(gap) || gap < 0) throw new Error("List gap must be finite and non-negative");
  const load = useAction(() => onLoadMore?.());
  const older = useAction(() => onLoadOlder?.());
  const committed = useRef<Records<T> | null>(null);
  const records = useMemo(() => {
    const previous = committed.current;
    const rows = new Map<string, RowRecord<T>>();
    let unchanged = items.length === previous?.data.length;
    let reordered = !unchanged;
    const changes: NativeItem[] = [];
    let data;
    try {
      data = items.map((item, index) => {
        const key = keyExtractor(item, index);
        if (typeof key !== "string" || rows.has(key)) throw new Error("List keys must be unique strings");
        const beforeRow = previous?.template === plan.template ? previous.rows.get(key) : undefined;
        const before = beforeRow?.native;
        const dependencies = plan.dependencies?.(item, index);
        const projected = before && sameDependencies(beforeRow?.dependencies, dependencies)
          ? before.values : nativeValue(plan.project(item, index), before?.values);
        const measurement = measurementKey ?? null;
        const record = before && before.values === projected && before.measurementKey === measurement
          ? before : { key, values: projected, measurementKey: measurement };
        rows.set(key, { item, index, dependencies, native: record });
        if (record !== previous?.data[index]) {
          unchanged = false;
          if (record !== before) changes.push(record);
          if (key !== previous?.data[index]?.key) reordered = true;
        }
        return record;
      });
    } catch (error) {
      if (error !== needsReact) throw error;
      return null;
    }
    if (unchanged && previous) data = previous.data;
    else if (previous) rememberListPatch(data, { previous: previous.data, changes,
      ...(reordered ? { keys: data.map(item => item.key) } : {}) });
    return { data, rows, template: plan.template };
  }, [items, keyExtractor, plan, measurementKey]);
  useLayoutEffect(() => { committed.current = records; }, [records]);
  const targets = useMemo(() => {
    type Template = { id: number; props: Record<string, unknown>; children: Template[] };
    const targets = new Map<number, Record<string, (string | number)[]>>();
    const visit = (node: Template) => {
      const events: Record<string, (string | number)[]> = {};
      for (const [name, value] of Object.entries(node.props)) {
        if (name.startsWith("on") && typeof value === "object" && value !== null && "path" in value) {
          const path = value.path;
          if (Array.isArray(path) && path.every(part => typeof part === "string" || typeof part === "number")) events[name] = path.slice(1);
        }
      }
      targets.set(node.id, events);
      node.children.forEach(visit);
    };
    visit(JSON.parse(plan.template) as Template);
    return targets;
  }, [plan.template]);
  if (!records) return createElement(ReactList<T>, props);
  const nativeProps = {
    items: records.data, keyField: "key", template: plan.template, gap, followEnd, initialEnd,
    hasMore: hasMore && load.status !== "pending" && load.status !== "error",
    hasOlder: hasOlder && older.status !== "pending" && older.status !== "error",
    onEndReached: onLoadMore ? load.run : undefined,
    onStartReached: onLoadOlder ? older.run : undefined,
    onRowEvent(key: string, target: number, name: string, ...args: unknown[]) {
      const path = targets.get(target)?.[name];
      const row = records.rows.get(key);
      let callback: unknown = row && path ? plan.project(row.item, row.index) : undefined;
      for (const part of path ?? []) {
        callback = callback != null && typeof callback === "object" ? Reflect.get(callback, part) : undefined;
      }
      if (path && typeof callback === "function") callback(...args);
    },
  };
  return createElement(Fragment, null,
    older.status === "error" && createElement(ListLoadError, { message: older.error.message, onRetry: older.run }),
    createElement("NativeList", nativeProps),
    load.status === "error" && createElement(ListLoadError, { message: load.error.message, onRetry: load.run }),
  );
}

/** @internal Fixed native descriptions for Ink's own list components. */
export function nativeListTemplate(build: (
  node: (type: string, props: Record<string, unknown>, children?: NativeRowNode[]) => NativeRowNode,
  field: (...path: (string | number)[]) => unknown,
) => NativeRowNode): string {
  let next = 1;
  return JSON.stringify({ id: 0, type: "Stack", props: { gap: 0 }, children: [build(
    (type, props, children = []) => ({ id: next++, type, props, children }),
    (...path) => ({ $value: 0, path: ["values", 0, ...path] }),
  )] });
}
type NativeRowNode = { id: number; type: string; props: Record<string, unknown>; children: NativeRowNode[] };
