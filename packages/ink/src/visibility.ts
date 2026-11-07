import { options, type Component, type VNode } from "preact";
import { useEffect as effectSlot, useLayoutEffect as layoutSlot, useState } from "preact/hooks";
import type { DependencyList, EffectCallback, ReactNode } from "react";

type Phase = "layout" | "passive";
type Slot = { __c?: () => void; inkEffect?: Effect };
type Scope = { visible: boolean; committed: boolean; changed: boolean; parent?: Scope };
type Owner = Component<Record<string, unknown>> & {
  __v: Tree; __H?: { __: Slot[] }; __n: { __inkVisibility?: Scope };
  inkScope?: Scope; getChildContext?: () => { __inkVisibility: Scope };
};
type Tree = VNode<Record<string, unknown>> & { __?: Tree; __k?: (Tree | null)[]; __c?: Owner; __v?: number | null };
type Effect = {
  owner: Effects; slot: Slot; phase: Phase; callback: EffectCallback;
  deps?: DependencyList; nextDeps?: DependencyList; pending: boolean; active: boolean;
};
type Effects = { component: Owner; effects: Effect[]; rendering: Effect[]; visible: boolean; mounted: boolean };
const owners = new WeakMap<Owner, Effects>();
const pending = new Set<Effects>();
const boundaries = new Set<Tree>();
let current: Owner;
let index = 0;
let previousHook: typeof hooks.__h;
let passive: (() => void)[] = [];
let flushing = false;

// Preact 10.29's published option-hook ABI. Keep this with the renderer ABI.
const hooks = options as typeof options & {
  __r?: (vnode: Tree) => void;
  __h?: (component: Owner, index: number, type: number) => void;
  __c?: (vnode: Tree, queue: unknown[]) => void;
  __e: (error: unknown, vnode: Tree) => void;
  __s?: boolean;
};

/** Keep state and host nodes while disconnecting effects in the hidden subtree. */
export function Activity(this: Owner, { mode, children }: { mode: "visible" | "hidden"; children?: ReactNode }): ReactNode {
  if (!this.inkScope) {
    const scope = this.inkScope = { visible: mode === "visible", committed: mode === "visible", changed: false, parent: this.__n.__inkVisibility };
    const context = { __inkVisibility: scope };
    this.getChildContext = () => context;
    // Keep the last reconciled children while hidden. Preact retains the latest
    // props on this component, so showing it applies deferred parent updates.
    this.shouldComponentUpdate = next => {
      scope.visible = next.mode === "visible";
      return scope.visible;
    };
  }
  this.inkScope.visible = mode === "visible";
  return children;
}

export function isComponentVisible(component: Component): boolean {
  const owner = component as Owner;
  for (let scope = owner.inkScope ?? owner.__n?.__inkVisibility; scope; scope = scope.parent)
    if (!scope.visible) return false;
  return true;
}

export function activityChanged(component: Component): boolean {
  return (component as Owner).inkScope!.changed;
}

function effectHook(component: Owner, slot: number, type: number) {
  index = slot;
  previousHook?.(component, slot, type);
}

function changed(previous: DependencyList | undefined, next: DependencyList | undefined) {
  if (!previous || !next || previous.length !== next.length) return true;
  for (let i = 0; i < next.length; i++) if (!Object.is(previous[i], next[i])) return true;
  return false;
}

function useScreenEffect(callback: EffectCallback, deps: DependencyList | undefined, phase: Phase) {
  // Allocate a genuine Preact effect slot: Prefresh can clean it and reset it.
  // Ink owns its scheduling, so Preact must not also enqueue the effect.
  const skipped = hooks.__s;
  const hook = hooks.__h;
  previousHook = hook;
  hooks.__h = effectHook;
  hooks.__s = true;
  try { (phase === "layout" ? layoutSlot : effectSlot)(callback, deps); }
  finally { hooks.__s = skipped; hooks.__h = hook; previousHook = undefined; }
  const slot = current.__H!.__[index];
  let owner = owners.get(current);
  if (!owner) {
    owner = { component: current, effects: [], rendering: [], visible: isComponentVisible(current), mounted: true };
    owners.set(current, owner);
  }
  const effect = slot.inkEffect ??= { owner, slot, phase, callback, pending: true, active: false };
  effect.callback = callback;
  effect.nextDeps = deps;
  effect.pending = !effect.active || changed(effect.deps, deps);
  owner.rendering.push(effect);
}

export function useEffect(callback: EffectCallback, deps?: DependencyList) {
  useScreenEffect(callback, deps, "passive");
}

export function useLayoutEffect(callback: EffectCallback, deps?: DependencyList) {
  useScreenEffect(callback, deps, "layout");
}

function clean(effect: Effect) {
  const cleanup = effect.slot.__c;
  effect.slot.__c = undefined;
  effect.active = false;
  cleanup?.();
}

function start(effect: Effect) {
  if (!effect.owner.mounted || !effect.owner.visible || !effect.pending) return;
  effect.pending = false;
  effect.active = true;
  effect.deps = effect.nextDeps;
  const cleanup = effect.callback();
  // Prefresh invokes this same cleanup before refreshing or resetting hooks.
  effect.slot.__c = () => {
    effect.active = false;
    effect.deps = undefined;
    cleanup?.();
  };
}

function run(owner: Effects, phase: Phase, setup: boolean, all = false) {
  for (const effect of owner.effects) {
    if (effect.phase !== phase) continue;
    try {
      if (setup) start(effect);
      else if (effect.active && (all || effect.pending)) clean(effect);
    } catch (error) {
      hooks.__e(error, owner.component.__v);
    }
  }
}

function flushPassive() {
  if (flushing || !passive.length) return;
  const work = passive;
  passive = [];
  flushing = true;
  try { for (const callback of work) callback(); }
  finally { flushing = false; }
}

const beforeRender = hooks.__r;
hooks.__r = vnode => {
  flushPassive();
  beforeRender?.(vnode);
  current = vnode.__c!;
  const owner = owners.get(current);
  if (owner) owner.rendering = [];
};
const afterDiff = options.diffed;
options.diffed = vnode => {
  afterDiff?.(vnode);
  const tree = vnode as Tree;
  if (tree.__v === null) return;
  const owner = tree.__c && owners.get(tree.__c);
  if (owner && owner.rendering.length) {
    owner.effects = owner.rendering;
    pending.add(owner);
  }
  if (tree.type === Activity && tree.__c) {
    const scope = tree.__c.inkScope!;
    scope.changed = scope.committed !== scope.visible;
    scope.committed = scope.visible;
    if (scope.changed) boundaries.add(tree);
  }
};

function collect(tree: Tree, before: Effects[], after: Effects[]) {
  const owner = tree.__c && owners.get(tree.__c);
  if (owner) before.push(owner);
  for (const child of tree.__k ?? []) if (child) collect(child, before, after);
  if (owner) after.push(owner);
}

const commit = hooks.__c;
hooks.__c = (root, queue) => {
  let work = [...pending];
  const preorder: Effects[] = [];
  // A memoised child may not render when its Activity changes visibility.
  if (boundaries.size) {
    work = [];
    collect(root, preorder, work);
  }
  const hidden: Effects[] = [];
  for (const owner of preorder.length ? preorder : work) {
    const next = isComponentVisible(owner.component);
    if (next !== owner.visible) {
      owner.visible = next;
      if (!next) hidden.push(owner);
      else for (const effect of owner.effects) effect.pending = true;
    }
  }
  pending.clear();
  boundaries.clear();
  // Effect-event refs are commit bookkeeping and must update before cleanups.
  commit?.(root, queue);
  for (const owner of hidden) run(owner, "layout", false, true);
  for (const owner of work) if (owner.visible) run(owner, "layout", false);
  for (const owner of work) if (owner.visible) run(owner, "layout", true);
  if (hidden.length || work.length) {
    passive.push(() => {
      for (const owner of hidden) run(owner, "passive", false, true);
      for (const owner of work) if (owner.visible) run(owner, "passive", false);
      for (const owner of work) if (owner.visible) run(owner, "passive", true);
    });
    queueMicrotask(flushPassive);
  }
};

const unmount = options.unmount;
options.unmount = vnode => {
  const component = (vnode as Tree).__c;
  const owner = component && owners.get(component);
  if (owner) {
    owner.mounted = false;
    pending.delete(owner);
    run(owner, "layout", false, true);
    // Preact's unmount hook must not run passive cleanups in the layout phase.
    const cleanups = owner.effects.filter(effect => effect.phase === "passive" && effect.active)
      .map(effect => { const cleanup = effect.slot.__c; effect.slot.__c = undefined; effect.active = false; return cleanup; });
    passive.push(() => { for (const cleanup of cleanups) cleanup?.(); });
    queueMicrotask(flushPassive);
  }
  unmount?.(vnode);
};

/** Subscribe only while visible; read the current snapshot when reconnecting. */
export function useSyncExternalStore<T>(subscribe: (callback: () => void) => () => void, getSnapshot: () => T, _getServerSnapshot?: () => T): T {
  const value = getSnapshot();
  const [{ instance }, update] = useState(() => ({ instance: { value, getSnapshot } }));
  const check = () => {
    try { return !Object.is(instance.value, instance.getSnapshot()); }
    catch { return true; }
  };
  useLayoutEffect(() => {
    instance.value = value;
    instance.getSnapshot = getSnapshot;
    if (check()) update({ instance });
  }, [subscribe, value, getSnapshot]);
  useEffect(() => {
    if (check()) update({ instance });
    return subscribe(() => { if (check()) update({ instance }); });
  }, [subscribe]);
  return value;
}

export function useImperativeHandle<T>(ref: import("react").Ref<T> | undefined, create: () => T, deps?: DependencyList) {
  useLayoutEffect(() => {
    if (typeof ref === "function") {
      const cleanup = ref(create());
      return typeof cleanup === "function" ? cleanup : () => { ref(null); };
    }
    if (ref) {
      ref.current = create();
      return () => { ref.current = null; };
    }
  }, deps ? [...deps, ref] : undefined);
}
