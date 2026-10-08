import { createElement, Fragment, useLayoutEffect, useMemo, useRef, type ComponentProps, type ReactNode } from "./react";
import { ListLoadError, ReactList, type ListProps } from "./list";
import { useAction } from "./action";
import type { Row } from "./image";
import { pressHandler } from "./press";
import { rememberListPatch, type NativeListItem } from "./list-patch";

type Captured = { value: unknown; prototype?: object | null; fields?: [PropertyKey, boolean, Captured][] };
type Scope = { identity: string; values: readonly Captured[] };
type Capture = { identity: string; values: () => readonly unknown[]; usesIndex?: boolean };
type Plan<T> = { project: (item: T, index: number) => unknown[]; template: string; scope?: Capture };
const needsReact = /* @__PURE__ */ Symbol("list needs React");
const plans = /* @__PURE__ */ new WeakMap<Function, Plan<unknown>>();
const keys = /* @__PURE__ */ new WeakMap<Function, string>();
const plainRecords = /* @__PURE__ */ new WeakSet<object>();
declare const __inkIsProxy: (value: unknown) => boolean;
const isProxy = typeof __inkIsProxy === "function" ? __inkIsProxy : undefined;

function bareFunctionPrototype(value: unknown, owner: Function): value is object {
  if (value === null || typeof value !== "object" || isProxy?.(value)) return false;
  if (Object.getPrototypeOf(value) !== Object.prototype || Reflect.ownKeys(value).length !== 1) return false;
  const constructor = Object.getOwnPropertyDescriptor(value, "constructor");
  return !!constructor && "value" in constructor && constructor.value === owner;
}

function plainData(value: unknown, cache = plainRecords, visiting = new Set<object>()): boolean {
  if (value === null || typeof value !== "object" && typeof value !== "function") return true;
  if (cache.has(value)) return true;
  if (isProxy?.(value)) return false;
  if (visiting.has(value) || visiting.size >= 64) return false;
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== null && prototype !== Object.prototype && prototype !== Array.prototype
    && !(typeof value === "function" && prototype === Function.prototype)) return false;
  visiting.add(value);
  const properties: Record<PropertyKey, PropertyDescriptor> = Object.getOwnPropertyDescriptors(value);
  const plain = Reflect.ownKeys(properties).every(key => {
    const property = properties[key];
    if (!("value" in property)) return false;
    if (["toString", "valueOf", Symbol.toPrimitive].includes(key) && typeof property.value === "function") return false;
    if (key === "prototype" && typeof value === "function" && bareFunctionPrototype(property.value, value)) return true;
    return plainData(property.value, cache, visiting);
  });
  visiting.delete(value);
  if (plain && isProxy) cache.add(value);
  return plain;
}

/** @internal A compiler-proven field extractor has no captured state. */
export function nativeListKey<T>(extract: (item: T, index: number) => string, identity: string) {
  keys.set(extract, identity);
  return extract;
}

/** @internal Created by the Ink compiler; application List props stay unchanged. */
export function nativeListRow<T>(render: (item: T, index: number) => ReactNode, project: Plan<T>["project"], template: string, scope?: Capture) {
  plans.set(render, { project: project as Plan<unknown>["project"], template, scope });
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
  return { image: props.image, title: props.title, titleMaxLines: props.titleMaxLines,
    titleIcon: props.titleIcon, subtitle: props.subtitle, subtitleIcon: props.subtitleIcon,
    onLongPress: props.onLongPress,
    onPress: pressHandler(props) ?? (props.onLongPress ? () => {} : undefined) };
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
type RowRecord<T> = { item: T; index: number; native: NativeItem };
type Records<T> = { data: NativeItem[]; rows: Map<string, RowRecord<T>>; template: string;
  items: readonly T[]; key: string | Function; measurementKey?: string | number; scope?: Scope };

function captureValue(value: unknown, cache: WeakMap<object, Captured>): Captured {
  if (value === null || typeof value !== "object" && typeof value !== "function") return { value };
  const previous = cache.get(value);
  if (previous) return previous;
  const properties: Record<PropertyKey, PropertyDescriptor> = Object.getOwnPropertyDescriptors(value);
  const captured: Captured = { value, prototype: Object.getPrototypeOf(value), fields: Reflect.ownKeys(properties).map(key => {
    const property = properties[key];
    const field = key === "prototype" && typeof value === "function" && bareFunctionPrototype(property.value, value)
      ? { value: property.value } : captureValue(property.value, cache);
    return [key, !!property.enumerable, field];
  }) };
  cache.set(value, captured);
  return captured;
}

function sameCapture(before: Captured, after: Captured, seen: WeakMap<Captured, Captured>): boolean {
  // Other JS hosts cannot establish that an object has no Proxy traps.
  if (before.value !== null && (typeof before.value === "object" || typeof before.value === "function") && !isProxy) return false;
  if (!Object.is(before.value, after.value) || before.prototype !== after.prototype) return false;
  const previous = before.fields, current = after.fields;
  if (!previous || !current) return previous === current;
  if (seen.get(before) === after) return true;
  seen.set(before, after);
  return previous.length === current.length && current.every(([key, enumerable, value], index) =>
    key === previous[index][0] && enumerable === previous[index][1] && sameCapture(previous[index][2], value, seen));
}

function sameScopeValues(before: readonly Captured[], after: readonly Captured[]): boolean {
  const seen = new WeakMap<Captured, Captured>();
  return before.length === after.length && after.every((value, index) => sameCapture(before[index], value, seen));
}

export function nativeListPlan<T>(render: ListProps<T>["renderItem"]): Plan<T> | undefined {
  return plans.get(render) as Plan<T> | undefined;
}

export function CompiledList<T>({ plan, ...props }: ListProps<T> & { plan: Plan<T> }) {
  const { items, keyExtractor, gap = 47, followEnd, initialPosition, measurementKey, onLoadMore, onLoadOlder, hasMore = false, hasOlder = false } = props;
  if (!Number.isFinite(gap) || gap < 0) throw new Error("List gap must be finite and non-negative");
  const load = useAction(() => onLoadMore?.());
  const older = useAction(() => onLoadOlder?.());
  const committed = useRef<Records<T> | null>(null);
  const records = useMemo(() => {
    const previous = committed.current;
    const keyIdentity = keys.get(keyExtractor) ?? keyExtractor;
    const captured = plan.scope && (items.length ? plan.scope.values() : []);
    if (captured && !captured.every(value => plainData(value, new WeakSet()))) return null;
    const snapshots = new WeakMap<object, Captured>();
    const scope = plan.scope && { identity: plan.scope.identity, values: captured?.map(value => captureValue(value, snapshots)) ?? [] };
    const sameScope = scope && previous?.scope?.identity === scope.identity && sameScopeValues(previous.scope.values, scope.values);
    if (previous && previous.items === items && previous.key === keyIdentity
      && previous.template === plan.template && previous.measurementKey === measurementKey
      && sameScope) return previous;
    const rows = new Map<string, RowRecord<T>>();
    let unchanged = items.length === previous?.data.length;
    let reordered = !unchanged;
    const changes: NativeItem[] = [];
    let data;
    try {
      if (!items.every(item => plainData(item))) throw needsReact;
      data = items.map((item, index) => {
        const key = typeof keyIdentity === "string" && previous?.key === keyIdentity && previous.items[index] === item
          ? previous.data[index].key : keyExtractor(item, index);
        if (typeof key !== "string" || rows.has(key)) throw new Error("List keys must be unique strings");
        const beforeRow = previous?.template === plan.template ? previous.rows.get(key) : undefined;
        const before = beforeRow?.native;
        const projected = beforeRow && sameScope && beforeRow.item === item
          && (beforeRow.index === index || plan.scope?.usesIndex === false)
          ? beforeRow.native.values : nativeValue(plan.project(item, index), before?.values);
        const measurement = measurementKey ?? null;
        const record = before && before.values === projected && before.measurementKey === measurement
          ? before : { key, values: projected, measurementKey: measurement };
        rows.set(key, { item, index, native: record });
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
    return { data, rows, template: plan.template, items, key: keyIdentity, measurementKey, scope };
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
    items: records.data, keyField: "key", template: plan.template, gap, followEnd, initialEnd: initialPosition === "end",
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
  return JSON.stringify({ id: 0, type: "Stack", props: { gap: 0, align: "stretch" }, children: [build(
    (type, props, children = []) => ({ id: next++, type, props, children }),
    (...path) => ({ $value: 0, path: ["values", 0, ...path] }),
  )] });
}
type NativeRowNode = { id: number; type: string; props: Record<string, unknown>; children: NativeRowNode[] };
