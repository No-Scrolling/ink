import { createElement, Fragment, useLayoutEffect, useMemo, useRef, useState, type ComponentProps, type ReactNode } from "./react";
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
declare const __inkIsDenseArray: (value: unknown) => boolean;
const isDenseArray = typeof __inkIsDenseArray === "function" ? __inkIsDenseArray : undefined;

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
type Records<T> = { data: NativeItem[]; positions: Map<string, number>; template: string;
  items: readonly T[]; key: string | Function; measurementKey?: string | number; scope?: Scope };
const cooperativeListSize = 512;
const preparationBudget = 4;
const minimumPreparationRows = 32;
const maximumPreparationRows = 128;
type Preparation<T> = { steps: Generator<void, Records<T> | null>; result?: { value: Records<T> | null }; error?: { value: unknown } };
type Request<T> = { items: readonly T[]; key: string | Function; plan: Plan<T>;
  measurementKey?: string | number; scope?: Scope; preparation: Preparation<T>; synchronous?: { value: Records<T> | null };
  eventPositions: Map<string, number | undefined> };
type Prepared<T> = { request: Request<T>; generation: number } &
  ({ status: "ready"; value: Records<T> | null } | { status: "error"; error: unknown });

function listItem<T>(items: readonly T[], index: number): T {
  const property = Object.getOwnPropertyDescriptor(items, index);
  if (!property || !("value" in property)) throw needsReact;
  return property.value;
}

/**
 * Reproject only the items replaced in place, when every key stays at its position.
 * Returns undefined when keys move, so the caller rebuilds the whole list.
 */
function* replaceInPlace<T>(previous: Records<T>, items: readonly T[], plan: Plan<T>,
  keyExtractor: ListProps<T>["keyExtractor"], dense: boolean): Generator<void, Records<T> | undefined> {
  let data: NativeItem[] | undefined;
  const changes: NativeItem[] = [];
  const length = items.length;
  for (let index = 0; index < length; index++) {
    const item = dense ? items[index] : listItem(items, index);
    if (item === previous.items[index]) continue;
    const before = previous.data[index];
    if (!plainData(item)) throw needsReact;
    if (keyExtractor(item, index) !== before.key) return undefined;
    const projected = nativeValue(plan.project(item, index), before.values);
    if (projected !== before.values) {
      const record = { key: before.key, values: projected, measurementKey: before.measurementKey };
      (data ??= previous.data.slice())[index] = record;
      changes.push(record);
    }
    yield;
  }
  if (data) rememberListPatch(data, { previous: previous.data, changes });
  return { ...previous, data: data ?? previous.data, items };
}

function captureValue(value: unknown, cache: WeakMap<object, Captured>, visiting = new Set<object>()): Captured {
  if (value === null || typeof value !== "object" && typeof value !== "function") return { value };
  const previous = cache.get(value);
  if (previous && isProxy) return previous;
  if (isProxy?.(value) || visiting.has(value) || visiting.size >= 64) throw needsReact;
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== null && prototype !== Object.prototype && prototype !== Array.prototype
    && !(typeof value === "function" && prototype === Function.prototype)) throw needsReact;
  visiting.add(value);
  const properties: Record<PropertyKey, PropertyDescriptor> = Object.getOwnPropertyDescriptors(value);
  const fields: NonNullable<Captured["fields"]> = [];
  for (const key of Reflect.ownKeys(properties)) {
    const property = properties[key];
    if (!("value" in property)) throw needsReact;
    if (["toString", "valueOf", Symbol.toPrimitive].includes(key) && typeof property.value === "function") throw needsReact;
    const field = key === "prototype" && typeof value === "function" && bareFunctionPrototype(property.value, value)
      ? { value: property.value } : captureValue(property.value, cache, visiting);
    fields.push([key, !!property.enumerable, field]);
  }
  visiting.delete(value);
  const captured: Captured = { value, prototype, fields };
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

function captureScope<T>(plan: Plan<T>, items: readonly T[]): Scope | undefined {
  if (!plan.scope) return;
  const captured = items.length ? plan.scope.values() : [];
  // Captures can mutate between slices; validate each fresh snapshot without the row cache.
  return { identity: plan.scope.identity, values: captured.map(value => captureValue(value, new WeakMap())) };
}

function sameScope(before?: Scope, after?: Scope): boolean {
  return !!before && !!after && before.identity === after.identity && sameScopeValues(before.values, after.values);
}

function capturesMatch<T>(request: Request<T>, latestPlan: Plan<T> | null): boolean {
  return !!latestPlan && sameScope(request.scope, captureScope(request.plan, request.items))
    && (latestPlan === request.plan || sameScope(request.scope, captureScope(latestPlan, request.items)));
}

function* prepareRecords<T>(previous: Records<T> | null, items: readonly T[], plan: Plan<T>,
  keyExtractor: ListProps<T>["keyExtractor"], keyIdentity: string | Function,
  measurementKey: string | number | undefined, scope: Scope | undefined, dense: boolean): Generator<void, Records<T> | null> {
  try {
    if (isProxy?.(items) || Object.getPrototypeOf(items) !== Array.prototype) throw needsReact;
    const length = items.length;
    const unchangedScope = sameScope(previous?.scope, scope);
    if (previous && previous.items === items && previous.key === keyIdentity
      && previous.template === plan.template && previous.measurementKey === measurementKey
      && unchangedScope) return previous;
    if (previous && unchangedScope && typeof keyIdentity === "string" && previous.key === keyIdentity
      && previous.template === plan.template && previous.measurementKey === measurementKey
      && previous.items.length === length) {
      const patched = yield* replaceInPlace(previous, items, plan, keyExtractor, dense);
      if (patched !== undefined) return { ...patched, scope };
    }
    const positions = new Map<string, number>();
    let unchanged = length === previous?.data.length;
    let reordered = !unchanged;
    const changes: NativeItem[] = [];
    let data: NativeItem[] = [];
    const itemKeys: string[] = [];
    for (let index = 0; index < length; index++) {
      const item = dense ? items[index] : listItem(items, index);
      if (!plainData(item)) throw needsReact;
      const key = typeof keyIdentity === "string" && previous?.key === keyIdentity && previous.items[index] === item
        ? previous.data[index].key : keyExtractor(item, index);
      if (typeof key !== "string" || positions.has(key)) throw new Error("List keys must be unique strings");
      const beforeIndex = previous?.template === plan.template ? previous.positions.get(key) : undefined;
      const before = previous && beforeIndex !== undefined ? previous.data[beforeIndex] : undefined;
      const projected = previous && beforeIndex !== undefined && unchangedScope && previous.items[beforeIndex] === item
        && (beforeIndex === index || plan.scope?.usesIndex === false)
        ? previous.data[beforeIndex].values : nativeValue(plan.project(item, index), before?.values);
      const measurement = measurementKey ?? null;
      const record = before && before.values === projected && before.measurementKey === measurement
        ? before : { key, values: projected, measurementKey: measurement };
      positions.set(key, index);
      itemKeys.push(key);
      if (record !== previous?.data[index]) {
        unchanged = false;
        if (record !== before) changes.push(record);
        if (key !== previous?.data[index]?.key) reordered = true;
      }
      data.push(record);
      yield;
    }
    if (unchanged && previous) data = previous.data;
    else if (previous) rememberListPatch(data, { previous: previous.data, changes,
      ...(reordered ? { keys: itemKeys } : {}) });
    return { data, positions, template: plan.template, items, key: keyIdentity, measurementKey, scope };
  } catch (error) {
    if (error !== needsReact) throw error;
    return null;
  }
}

function advancePreparation<T>(preparation: Preparation<T>, cooperative: boolean) {
  if (preparation.result) return preparation.result;
  if (preparation.error) throw preparation.error.value;
  const started = cooperative ? performance.now() : 0;
  let rows = 0;
  while (true) {
    let step;
    try {
      step = preparation.steps.next();
    } catch (error) {
      preparation.error = { value: error };
      throw error;
    }
    if (step.done) return preparation.result = { value: step.value };
    rows++;
    // Identity-only sparse scans do not yield: a single replaced row stays synchronous.
    if (cooperative && (rows >= maximumPreparationRows
      || rows >= minimumPreparationRows && performance.now() - started >= preparationBudget)) return;
  }
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
  const committedRequest = useRef<Request<T> | null>(null);
  const committedPlan = useRef<Plan<T> | null>(null);
  const generation = useRef(0);
  const [prepared, setPrepared] = useState<Prepared<T> | null>(null);
  const request = useMemo<Request<T>>(() => {
    const keyIdentity = keys.get(keyExtractor) ?? keyExtractor;
    let scope: Scope | undefined;
    try {
      scope = captureScope(plan, items);
    } catch (error) {
      if (error !== needsReact) throw error;
      const preparation: Preparation<T> = { steps: (function* () { return null; })(), result: { value: null } };
      return { items, key: keyIdentity, plan, measurementKey, preparation, synchronous: preparation.result, eventPositions: new Map() };
    }
    const pending = committedRequest.current;
    if (pending && pending.items === items && pending.key === keyIdentity
      && pending.plan.template === plan.template && pending.measurementKey === measurementKey
      && sameScope(pending.scope, scope)) return pending;
    const previous = committed.current;
    const dense = isDenseArray?.(items) ?? false;
    const preparation: Preparation<T> = { steps: prepareRecords(previous, items, plan, keyExtractor, keyIdentity, measurementKey, scope, dense) };
    // First population must exist when List's one-shot initialPosition effect scrolls its Screen.
    const cooperative = !!isProxy && !!scope && typeof keyIdentity === "string"
      && !!previous?.items.length && previous.template === plan.template && items.length >= cooperativeListSize;
    const synchronous = advancePreparation(preparation, cooperative);
    return { items, key: keyIdentity, plan, measurementKey, scope, preparation, synchronous, eventPositions: new Map() };
  }, [items, keyExtractor, plan, measurementKey]);
  const outcome = prepared?.request === request && prepared.generation === generation.current ? prepared : undefined;
  let records = outcome?.status === "ready" ? outcome.value
    : request.synchronous ? request.synchronous.value : committed.current;
  if (outcome?.status === "ready" && !request.synchronous) {
    try {
      if (!capturesMatch(request, committedPlan.current)) records = null;
    } catch (error) {
      if (error !== needsReact) throw error;
      records = null;
    }
  }
  const preparing = !request.synchronous && outcome?.status !== "ready";
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
  const events = useRef<{
    items: readonly T[]; keyExtractor: ListProps<T>["keyExtractor"]; plan: Plan<T>;
    targets: typeof targets; positions: Map<string, number | undefined>;
  } | null>(null);
  useLayoutEffect(() => {
    committed.current = records;
    committedRequest.current = request;
    committedPlan.current = plan;
    events.current = { items, keyExtractor, plan, targets, positions: request.eventPositions };
    return () => {
      events.current = null;
      committedRequest.current = null;
      committedPlan.current = null;
    };
  });
  useLayoutEffect(() => {
    const currentGeneration = ++generation.current;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const current = () => generation.current === currentGeneration && committedRequest.current === request;
    const run = () => {
      if (!current()) return;
      try {
        const result = capturesMatch(request, committedPlan.current)
          ? advancePreparation(request.preparation, true) : { value: null };
        if (!current()) return;
        if (result) setPrepared({ request, generation: currentGeneration, status: "ready", value: result.value });
        else timer = setTimeout(run, 0);
      } catch (error) {
        if (!current()) return;
        setPrepared(error === needsReact
          ? { request, generation: currentGeneration, status: "ready", value: null }
          : { request, generation: currentGeneration, status: "error", error });
      }
    };
    if (!request.synchronous) timer = setTimeout(run, 0);
    return () => {
      generation.current++;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [request]);
  if (outcome?.status === "error") throw outcome.error;
  if (!records) return createElement(ReactList<T>, props);
  const nativeProps = {
    items: records.data, keyField: "key", template: records.template, gap, followEnd, initialEnd: initialPosition === "end",
    hasMore: !preparing && hasMore && load.status !== "pending" && load.status !== "error",
    hasOlder: !preparing && hasOlder && older.status !== "pending" && older.status !== "error",
    onEndReached: !preparing && onLoadMore ? load.run : undefined,
    onStartReached: !preparing && onLoadOlder ? older.run : undefined,
    onRowEvent(key: string, target: number, name: string, ...args: unknown[]) {
      const latest = events.current;
      const path = latest?.targets.get(target)?.[name];
      if (!latest || !path || isProxy?.(latest.items)) return;
      const matches = (index: number) => {
        const property = Object.getOwnPropertyDescriptor(latest.items, index);
        return !!property && "value" in property && plainData(property.value)
          && latest.keyExtractor(property.value, index) === key;
      };
      let index = latest.items === records.items ? records.positions.get(key) : latest.positions.get(key);
      if (latest.items !== records.items && !latest.positions.has(key)) {
        const beforeIndex = records.positions.get(key);
        if (beforeIndex !== undefined && matches(beforeIndex)) index = beforeIndex;
        else {
          for (let candidate = 0; candidate < latest.items.length; candidate++) {
            if (matches(candidate)) { index = candidate; break; }
          }
        }
        latest.positions.set(key, index);
      }
      if (index === undefined) return;
      let callback: unknown;
      try {
        const item = listItem(latest.items, index);
        if (!plainData(item)) return;
        callback = latest.plan.project(item, index);
      } catch (error) {
        if (error === needsReact) return;
        throw error;
      }
      for (const part of path) {
        callback = callback != null && typeof callback === "object" ? Reflect.get(callback, part) : undefined;
      }
      if (typeof callback === "function") callback(...args);
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
