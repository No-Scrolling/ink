import { createElement, useLayoutEffect, useState } from "./react";
import { allocateHostId, cancelViewUpdate, registerHostActions, scheduleViewUpdate, type ViewData as Data } from "./host";

type Action = (...args: unknown[]) => void;
type Property = Data | Binding<Data> | Action | undefined;
type Kind = "Screen" | "Stack" | "Text" | "Button" | "TextInput" | "Toggle" | "Image" | "Icon"
  | "NativeList" | "PlayingScreen" | "Pressable" | "PlayingLayout" | "PlayingLabel" | "PlayingTransport" | "PlayingProgress" | "PitchIndicator";
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

/** A mounted native view's JavaScript state and actions; it does not retain a host tree. */
export class ViewScope {
  private readonly id = allocateHostId();
  private readonly values = new Map<number, ViewValue<Data>>();
  private readonly dirty = new Set<number>();
  private readonly derived: (() => void)[] = [];
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
      for (const id of this.dirty) values.push([id, this.values.get(id)!.get()]);
      this.dirty.clear();
      return { op: "values", view: this.id, values };
    });
  }

  /** @internal */
  initialise(build: (scope: ViewScope) => Description) {
    this.constructing();
    const root = build(this);
    this.initialised = true;
    return JSON.stringify({ view: this.id, root, values: [...this.values].map(([id, value]) => [id, value.get()]) });
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
