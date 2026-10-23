type Primitive = string | number | boolean | null;
type StringField<T> = { [K in keyof T]-?: T[K] extends string ? K : never }[keyof T] & string;
export type SourceWhere<T> = { [K in keyof T]?: Exclude<T[K], undefined> extends Primitive ? Exclude<T[K], undefined> : never };

export interface SourceSnapshot<T> {
  readonly revision: number;
  readonly count: number;
  get(key: string): Readonly<T> | undefined;
  at(index: number): Readonly<T> | undefined;
}

export interface SourceView<T> {
  getSnapshot(): SourceSnapshot<T>;
  subscribe(listener: () => void): () => void;
  dispose(): void;
}

export interface Source<T> extends SourceView<T> {
  set(record: T): void;
  transact(records: readonly T[], baseRevision?: number): void;
  replace(records: readonly T[], baseRevision?: number): void;
  view(where: SourceWhere<T>): SourceView<T>;
}

type Descriptor = { id: number; revision: number; count: number };
type NativeSnapshot = {
  revision: number;
  count: number;
  get(key: string): string | null;
  at(index: number): string | null;
};
type Entry = { id: number; closed: boolean };
const handles = new WeakMap<object, Entry>();
declare const __inkSource: (operation: string, payload: string) => string;
declare const __inkSourceSnapshot: (view: number) => NativeSnapshot;
declare const __inkIsProxy: (value: unknown) => boolean;

function encode(value: unknown): string {
  const visiting = new Set<object>();
  const visit = (item: unknown, depth: number) => {
    if (item === null || typeof item === "string" || typeof item === "boolean") return;
    if (typeof item === "number" && Number.isFinite(item)) return;
    if (typeof item !== "object" || depth > 64 || visiting.has(item) || __inkIsProxy(item)) {
      throw new TypeError("Sources require finite, acyclic plain JSON data");
    }
    const prototype = Object.getPrototypeOf(item);
    if (prototype !== Object.prototype && prototype !== null && prototype !== Array.prototype) {
      throw new TypeError("Source records must use plain objects and arrays");
    }
    if ("toJSON" in item) throw new TypeError("Source data cannot contain toJSON methods");
    const array = Array.isArray(item);
    let elements = 0;
    visiting.add(item);
    for (const key of Reflect.ownKeys(item)) {
      const property = Object.getOwnPropertyDescriptor(item, key)!;
      if (key === "toJSON" || !("value" in property) || typeof key === "symbol") {
        throw new TypeError("Source data cannot contain accessors, symbols or toJSON methods");
      }
      if (property.enumerable) {
        if (array && (String(Number(key)) !== key || !Number.isInteger(Number(key)) || Number(key) < 0 || Number(key) >= item.length)) {
          throw new TypeError("Source arrays require consecutive elements without extra properties");
        }
        elements++;
        visit(property.value, depth + 1);
      }
    }
    if (array && elements !== item.length) throw new TypeError("Source arrays cannot contain missing elements");
    visiting.delete(item);
  };
  visit(value, 0);
  return JSON.stringify(value);
}

function request<T>(operation: string, payload: unknown): T {
  return JSON.parse(__inkSource(operation, encode(payload))) as T;
}

function freeze<T>(value: T): T {
  if (value !== null && typeof value === "object") {
    for (const child of Object.values(value)) freeze(child);
    Object.freeze(value);
  }
  return value;
}

function snapshot<T>(id: number): SourceSnapshot<T> {
  const native = __inkSourceSnapshot(id);
  const decode = (encoded: string | null): Readonly<T> | undefined =>
    encoded === null ? undefined : freeze(JSON.parse(encoded) as T);
  return Object.freeze({
    revision: native.revision,
    count: native.count,
    get: (key: string) => decode(native.get(key)),
    at: (index: number) => decode(native.at(index)),
  });
}

/** @internal Native list binding; records never travel through React props. */
export function sourceViewId(view: SourceView<unknown>): number {
  const entry = handles.get(view);
  if (!entry || entry.closed) throw new Error("SourceList requires an open source or view");
  return entry.id;
}

export function createSource<T extends object>(records: readonly T[], options: { key: StringField<T> }): Source<T> {
  if (typeof __inkSource !== "function") throw new Error("Native sources require an updated Ink runtime");
  const created = request<Descriptor>("create", { key: options.key, items: records });
  let revision = created.revision;
  const views = new Map<number, {
    invalidate: () => void;
    close: () => void;
    listeners: Set<() => void>;
  }>();

  function makeView(descriptor: Descriptor): SourceView<T> {
    const entry = { id: descriptor.id, closed: false };
    const listeners = new Set<() => void>();
    let current: SourceSnapshot<T> | undefined;
    const assertOpen = () => { if (entry.closed) throw new Error("Source view is disposed"); };
    const view: SourceView<T> = Object.freeze({
      getSnapshot() { assertOpen(); return current ??= snapshot<T>(entry.id); },
      subscribe(listener: () => void) {
        assertOpen();
        listeners.add(listener);
        return () => { listeners.delete(listener); };
      },
      dispose() {
        if (entry.closed) return;
        if (entry.id === created.id) {
          request("release", { id: created.id });
          for (const child of views.values()) child.close();
          views.clear();
        } else {
          request("release", { id: entry.id });
          entry.closed = true;
          listeners.clear();
          views.delete(entry.id);
        }
      },
    });
    handles.set(view, entry);
    views.set(entry.id, {
      listeners,
      invalidate: () => { current = undefined; },
      close: () => { entry.closed = true; listeners.clear(); current = undefined; },
    });
    return view;
  }

  const base = makeView(created);
  function mutate(operation: "set" | "replace", items: readonly T[], baseRevision = revision) {
    sourceViewId(base);
    const result = request<{ revision: number; views: Descriptor[] }>(operation,
      { source: created.id, baseRevision, items });
    revision = result.revision;
    const changed = result.views.flatMap(view => {
      const state = views.get(view.id);
      if (!state) return [];
      state.invalidate();
      return [state];
    });
    const errors: unknown[] = [];
    for (const state of changed) for (const listener of [...state.listeners]) {
      try { listener(); } catch (error) { errors.push(error); }
    }
    if (errors.length) throw errors[0];
  }
  const source: Source<T> = Object.freeze({
    ...base,
    set: (record: T) => mutate("set", [record]),
    transact: (items: readonly T[], baseRevision?: number) => mutate("set", items, baseRevision),
    replace: (items: readonly T[], baseRevision?: number) => mutate("replace", items, baseRevision),
    view(where: SourceWhere<T>) {
      sourceViewId(base);
      return makeView(request<Descriptor>("view", { source: created.id, where }));
    },
  });
  handles.set(source, handles.get(base)!);
  return source;
}

export type SourceTextField<T> = StringField<T>;
