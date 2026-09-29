import { createElement, useLayoutEffect, useState } from "./react";
import { allocateHostId, cancelViewUpdate, registerHostActions, scheduleViewUpdate, type CollectionEdit, type CollectionPatch, type ViewData as Data } from "./host";
import { Expression } from "./expression";
export { expression } from "./expression";

type Action = (...args: unknown[]) => void;
type Property = Data | Binding<Data> | Action | undefined;
type Kind = "Screen" | "Stack" | "Text" | "Button" | "TextInput" | "Toggle" | "Image" | "Icon"
  | "NativeList" | "RowContent" | "Avatar" | "PlayingScreen" | "Pressable" | "PlayingLayout" | "PlayingLabel" | "PlayingTransport" | "PlayingProgress" | "PitchIndicator";
type Description = { id: number; type: Kind; props: Record<string, Data>; children: Description[] };

export class Binding<T extends Data> {
  constructor(readonly source: number, readonly path: readonly (string | number)[] = []) {}

  at<K extends keyof T & (string | number)>(key: K): Binding<T[K] & Data> {
    return new Binding(this.source, [...this.path, key]);
  }

  toJSON() { return { $value: this.source, path: this.path }; }
}

export class ViewValue<T extends Data> extends Binding<T> {
  constructor(source: number, private current: T, private changed: () => void) { super(source); }
  get(): T { return this.current; }
  set(value: T) {
    if (Object.is(this.current, value)) return;
    this.current = value;
    this.changed();
  }
}

/** Keyed data retained in Rust; mutations send only the changed records. */
export class ViewCollection<T extends { readonly [key: string]: Data }> extends Binding<readonly T[]> {
  private revision = 0;
  constructor(source: number, readonly destination: { view: number; source: number },
    private edit: (revision: number, value: CollectionEdit) => void) { super(source); }
  private change(value: CollectionEdit) { this.edit(++this.revision, value); }
  insert(item: T, before: string | null = null) { this.change({ op: "insert", item, before }); }
  update(key: string, value: Partial<T>) { this.change({ op: "update", key, value }); }
  remove(key: string) { this.change({ op: "remove", key }); }
  move(key: string, before: string | null = null) { this.change({ op: "move", key, before }); }
  reverse() { this.change({ op: "reverse" }); }
  reset(items: readonly T[]) { this.change({ op: "reset", items }); }
  /** @internal Reserve a revision for an asynchronous native query replacement. */
  reserve() { return { ...this.destination, revision: ++this.revision }; }
}

/** A mounted native view's JavaScript state and actions; it does not retain a host tree. */
export class ViewScope {
  private readonly id = allocateHostId();
  private readonly values = new Map<number, ViewValue<Data>>();
  private readonly dirty = new Set<number>();
  private readonly derived: (() => void)[] = [];
  private readonly expressions: { source: number; expression: Data }[] = [];
  private readonly collections: { source: number; key: string; items: readonly Data[] }[] = [];
  private readonly patches = new Map<number, CollectionPatch>();
  private readonly actions = new Map<number, Record<string, Action>>();
  private readonly starts: (() => void | (() => void))[] = [];
  private stops: (() => void)[] = [];
  private active = false;
  private evaluating = false;
  private initialised = false;

  private constructing() {
    if (this.initialised) throw new Error("Declare native controls and bindings inside nativeView's build callback");
  }

  value(initial: number): ViewValue<number>;
  value(initial: string): ViewValue<string>;
  value(initial: boolean): ViewValue<boolean>;
  value<T extends Data>(initial: T): ViewValue<T>;
  value<T extends Data>(initial: T): ViewValue<T> {
    this.constructing();
    const id = allocateHostId();
    const value = new ViewValue(id, initial, () => { this.dirty.add(id); this.schedule(); });
    this.values.set(id, value);
    return value;
  }

  derive<T extends Data>(dependencies: readonly Binding<Data>[], calculate: () => T): Binding<T> & { get(): T } {
    const value = this.value(calculate());
    this.derived.push(() => {
      if (dependencies.some(dependency => this.dirty.has(dependency.source))) value.set(calculate());
    });
    return value;
  }

  compute<T extends Data>(expression: Expression<T>): Binding<T> {
    this.constructing();
    const source = allocateHostId();
    this.expressions.push({ source, expression: expression.definition });
    return new Binding<T>(source);
  }

  collection<T extends { readonly [key: string]: Data }>(items: readonly T[], key: keyof T & string): ViewCollection<T> {
    this.constructing();
    const source = allocateHostId();
    this.collections.push({ source, key, items });
    return new ViewCollection(source, { view: this.id, source }, (revision, edit) => {
      let patch = this.patches.get(source);
      if (!patch) this.patches.set(source, patch = { source, revision, edits: [] });
      patch.revision = revision;
      if (edit.op === "reset") patch.edits.length = 0;
      patch.edits.push(edit);
      this.dirty.add(source);
      this.schedule();
    });
  }

  onMount(start: () => void | (() => void)) { this.constructing(); this.starts.push(start); }

  node(type: Kind, props: Readonly<Record<string, Property>>, ...children: Description[]): Description {
    this.constructing();
    const id = allocateHostId();
    const native: Record<string, Data> = {};
    const actions: Record<string, Action> = {};
    for (const [key, value] of Object.entries(props)) {
      if (typeof value === "function") { actions[key] = value; native[key] = true; }
      else if (value instanceof Binding) native[key] = value.toJSON();
      else if (value !== undefined) native[key] = value;
    }
    if (Object.keys(actions).length) this.actions.set(id, actions);
    return { id, type, props: native, children };
  }

  list<T extends { readonly [key: string]: Data }>(items: Binding<readonly T[]>,
    options: { key: keyof T & string; gap?: number; followEnd?: boolean; initialEnd?: boolean;
      onEndReached?: Action; onStartReached?: Action; hasMore?: boolean | Binding<boolean>; hasOlder?: boolean | Binding<boolean> },
    row: (item: Binding<T>) => Description): Description {
    this.constructing();
    const template = this.node("Stack", { gap: 0 }, row(new Binding<T>(0)));
    return this.node("NativeList", { ...options, items, template: JSON.stringify(template) });
  }

  private schedule() {
    if (!this.active || this.evaluating) return;
    scheduleViewUpdate(this.id, () => {
      if (!this.active || !this.dirty.size) return;
      this.evaluating = true;
      try { for (const update of this.derived) update(); }
      finally { this.evaluating = false; }
      const values: [number, Data][] = [];
      for (const id of this.dirty) {
        const value = this.values.get(id);
        if (value) values.push([id, value.get()]);
      }
      const collections = [...this.patches.values()];
      this.patches.clear();
      this.dirty.clear();
      return { op: "values", view: this.id, values, ...(collections.length ? { collections } : {}) };
    });
  }

  /** @internal */
  initialise(build: (scope: ViewScope) => Description) {
    this.constructing();
    const root = build(this);
    this.initialised = true;
    const definition = JSON.stringify({ view: this.id, root, values: [...this.values].map(([id, value]) => [id, value.get()]),
      expressions: this.expressions, collections: this.collections });
    this.collections.length = 0;
    this.expressions.length = 0;
    return definition;
  }

  /** @internal */
  attach() {
    this.active = true;
    this.stops = [...this.actions].map(([id, actions]) => registerHostActions(id, actions));
    const detach = () => {
      this.active = false;
      cancelViewUpdate(this.id);
      const errors: unknown[] = [];
      for (const stop of this.stops.splice(0).reverse()) {
        try { stop(); } catch (error) { errors.push(error); }
      }
      if (errors.length) throw new AggregateError(errors, "Native view cleanup failed");
    };
    try {
      for (const start of this.starts) {
        const stop = start();
        if (stop) this.stops.push(stop);
      }
      // Activity may reconnect after state changed while its view was hidden.
      if (this.dirty.size) this.schedule();
    } catch (error) {
      try { detach(); } catch (cleanup) { throw new AggregateError([error, cleanup], "Native view mount failed"); }
      throw error;
    }
    return detach;
  }
}

/** Declare native controls once per mount. React owns the page's lifetime, not its updates. */
export function nativeView(build: (scope: ViewScope) => Description) {
  return function NativeView() {
    const [view] = useState(() => {
      const scope = new ViewScope();
      return { scope, definition: scope.initialise(build) };
    });
    useLayoutEffect(() => view.scope.attach(), [view]);
    return createElement("NativeView", { definition: view.definition });
  };
}
