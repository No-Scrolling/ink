declare const process: { env: { NODE_ENV: string } };
import { createContext, type ReactNode } from "./react";
import Reconciler from "react-reconciler";
import { ConcurrentRoot, DefaultEventPriority, DiscreteEventPriority } from "react-reconciler/constants";
import { onNativeMessage } from "./native";
import { openLink } from "./external";
import { listPatch, type NativeListItem } from "./list-patch";
import { allocateHostId, dispatchHostAction, takeViewUpdates, type ViewUpdate } from "./host";

declare const __inkCommit: (operations: Operation[]) => void;
declare const __inkPost: (message: string) => void;

type Props = Record<string, unknown>;
type Instance = { id: number; type: string; props: Props; children: Instance[]; hidden?: boolean };
type Container = { id: number; children: Instance[] };
type Operation =
  | ViewUpdate
  | { op: "create"; id: number; type: string; props: Props }
  | { op: "update"; id: number; props: Props }
  | { op: "text"; changes: (number | string)[] }
  | { op: "insert"; id: number; parent: number; before: number | null }
  | { op: "remove"; id: number; parent: number }
  | { op: "hidden"; id: number; value: boolean };

const mounted = new Map<number, Instance>();
let operations: Operation[] = [];
let textChanges: (number | string)[] | undefined;
let commitScheduled = false;
let priority = DefaultEventPriority;
const hostContext = {};

function queue(operation: Operation, preserveTextBatch = false) {
  if (!preserveTextBatch) textChanges = undefined;
  operations.push(operation);
}

function createTextBatch() {
  const changes: (number | string)[] = [];
  operations.push({ op: "text", changes });
  return textChanges = changes;
}

function textContent(type: string, props: Props): string | null {
  if (type !== "Text") return null;
  const value = props.children;
  return typeof value === "string" ? value : typeof value === "number" || typeof value === "bigint" ? String(value) : null;
}

function hostProps(type: string, props: Props): Props {
  if (type !== "Text" || props.href === undefined) return props;
  const { href, ...rest } = props;
  return { ...rest, onPress: () => openLink(href as string) };
}

function nativeValue(value: unknown): unknown {
  return typeof value === "function" ? true : value;
}

function nativeProps(props: Props, type: string): Props {
  const result: Props = {};
  for (const name of Object.keys(props)) {
    if (name === "children" || name === "ref") continue;
    const value = props[name];
    if (value === undefined) continue;
    result[name] = typeof value === "function" ? true : value;
  }
  const text = textContent(type, props);
  if (text !== null) result.text = text;
  return result;
}

function publish(instance: Instance) {
  if (mounted.has(instance.id)) return;
  mounted.set(instance.id, instance);
  queue({ op: "create", id: instance.id, type: instance.type, props: nativeProps(instance.props, instance.type) });
  if (instance.hidden) queue({ op: "hidden", id: instance.id, value: true });
  for (const child of instance.children) {
    publish(child);
    queue({ op: "insert", id: child.id, parent: instance.id, before: null });
  }
}

function insert(parent: Container, child: Instance, before?: Instance) {
  const previous = parent.children.indexOf(child);
  if (previous !== -1) parent.children.splice(previous, 1);
  const index = before ? parent.children.indexOf(before) : parent.children.length;
  parent.children.splice(index, 0, child);
  publish(child);
  queue({ op: "insert", id: child.id, parent: parent.id, before: before?.id ?? null });
}

function forget(instance: Instance) {
  mounted.delete(instance.id);
  for (const child of instance.children) forget(child);
}

function remove(parent: Container, child: Instance) {
  parent.children.splice(parent.children.indexOf(child), 1);
  queue({ op: "remove", id: child.id, parent: parent.id });
  forget(child);
}

function sameListValue(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (a === null || b === null || typeof a !== "object" || typeof b !== "object") return false;
  if (Array.isArray(a)) return Array.isArray(b) && a.length === b.length && a.every((value, index) => sameListValue(value, b[index]));
  if (Array.isArray(b)) return false;
  const previous = a as Props, next = b as Props;
  const keys = Object.keys(previous);
  return keys.length === Object.keys(next).length && keys.every(key => Object.hasOwn(next, key) && sameListValue(previous[key], next[key]));
}

function compiledListUpdate(previous: Props, props: Props): Props | null {
  type Item = NativeListItem;
  const before = previous.items as Item[], after = props.items as Item[];
  const patch = listPatch(before, after);
  let byKey: Map<string, Item> | undefined;
  let reordered = patch ? patch.keys !== undefined : before.length !== after.length;
  const changes = patch?.changes ?? [];
  for (let index = 0; !patch && before !== after && index < after.length; index++) {
    const item = after[index];
    let old: Item | undefined = before[index];
    if (old?.key !== item.key) {
      reordered = true;
      byKey ??= new Map(before.map(item => [item.key, item]));
      old = byKey.get(item.key);
    }
    if (!old || !sameListValue(old.measurementKey, item.measurementKey) || !sameListValue(old.values, item.values)) changes.push(item);
  }
  const next = nativeProps(props, "NativeList");
  delete next.items;
  delete next.template;
  const metadataChanged = Object.keys({ ...previous, ...props }).some(name =>
    name !== "items" && name !== "template" && !Object.is(nativeValue(previous[name]), nativeValue(props[name])));
  if (!changes.length && !reordered && !metadataChanged) return null;
  next.itemChanges = changes;
  if (reordered) next.itemKeys = patch?.keys ?? after.map(item => item.key);
  return next;
}

function update(instance: Instance, type: string, _oldProps: Props, props: Props) {
  const previous = instance.props;
  const isText = type === "Text";
  if (isText && props.href !== undefined) props = hostProps(type, props);
  instance.props = props;
  if (type === "NativeList" && props.keyField === "key" && previous.template === props.template) {
    const next = compiledListUpdate(previous, props);
    if (next) queue({ op: "update", id: instance.id, props: next }, true);
    return;
  }
  if (isText) {
    if (previous.size !== props.size || previous.width !== props.width
      || previous.align !== props.align || previous.maxLines !== props.maxLines
      || previous.tabularNumbers !== props.tabularNumbers
      || (previous.onPress !== props.onPress && nativeValue(previous.onPress) !== nativeValue(props.onPress))) {
      queue({ op: "update", id: instance.id, props: nativeProps(props, type) });
      return;
    }
    const children = props.children;
    if (children !== previous.children) {
      if (typeof children === "string") return (textChanges ?? createTextBatch()).push(instance.id, children);
      if (typeof children === "number" || typeof children === "bigint") return (textChanges ?? createTextBatch()).push(instance.id, String(children));
    }
    return;
  }
  let previousCount = 0;
  let changed = false;
  for (const name of Object.keys(previous)) {
    if (name === "children" || name === "ref") continue;
    const value = previous[name];
    if (value === undefined) continue;
    previousCount++;
    const next = props[name];
    if (!Object.is(
      typeof value === "function" ? true : value,
      typeof next === "function" ? true : next,
    ) || !Object.hasOwn(props, name)) {
      changed = true;
      break;
    }
  }
  if (!changed) {
    let nextCount = 0;
    for (const name of Object.keys(props)) {
      if (name !== "children" && name !== "ref" && props[name] !== undefined) nextCount++;
    }
    if (previousCount === nextCount) return;
  }
  const next = nativeProps(props, type);
  if (type === "List" && nativeValue(previous.revision) === next.revision
    && nativeValue(previous.keys) === next.keys && nativeValue(previous.contentVersions) === next.contentVersions) {
    delete next.keys;
    delete next.contentVersions;
  }
  // Other node props cannot overwrite a pending text value.
  queue({ op: "update", id: instance.id, props: next }, type !== "#text");
}

function hide(instance: Instance, value: boolean) {
  instance.hidden = value;
  if (mounted.has(instance.id)) queue({ op: "hidden", id: instance.id, value });
}

function errorMessage(error: unknown, info: { componentStack?: string | null }) {
  const message = error instanceof Error ? `${error.name}: ${error.message}\n${error.stack ?? ""}` : String(error);
  return info.componentStack ? `${message}\n${info.componentStack}` : message;
}

function report(error: unknown, info: { componentStack?: string | null }) {
  __inkPost(JSON.stringify({ type: "error", message: errorMessage(error, info) }));
}

function reportRecovered(error: unknown, info: { componentStack?: string | null }) {
  __inkPost(JSON.stringify({ type: "log", level: "error", message: errorMessage(error, info) }));
}

const reconciler = Reconciler<string, Props, Container, Instance, Instance, never, never, never,
  Instance, object, never, ReturnType<typeof setTimeout>, -1, null>({
  supportsMutation: true,
  supportsPersistence: false,
  supportsHydration: false,
  isPrimaryRenderer: true,
  supportsMicrotasks: true,
  createInstance: (type, props) => ({ id: allocateHostId(), type, props: hostProps(type, props), children: [] }),
  createTextInstance: text => ({ id: allocateHostId(), type: "#text", props: { text }, children: [] }),
  appendInitialChild: (parent, child) => { parent.children.push(child); },
  finalizeInitialChildren: () => false,
  shouldSetTextContent: (type, props) => {
    if (type !== "Text") return false;
    if (props.href !== undefined && props.onPress) {
      throw new Error("Text accepts either href or onPress");
    }
    const kind = typeof props.children;
    return kind === "string" || kind === "number" || kind === "bigint";
  },
  getRootHostContext: () => hostContext,
  getChildHostContext: () => hostContext,
  getPublicInstance: instance => instance,
  prepareForCommit: () => null,
  resetAfterCommit: () => {
    if (!operations.length || commitScheduled) return;
    commitScheduled = true;
    queueMicrotask(() => {
      // Revealing an Activity reconnects store subscriptions in passive effects.
      // Include their synchronous corrections before presenting the page.
      while (reconciler.flushPassiveEffects()) {}
      reconciler.flushSyncWork();
      const committed = operations;
      committed.push(...takeViewUpdates());
      operations = [];
      textChanges = undefined;
      commitScheduled = false;
      if (committed.length) __inkCommit(committed);
    });
  },
  preparePortalMount: () => {},
  scheduleTimeout: setTimeout,
  cancelTimeout: clearTimeout,
  noTimeout: -1,
  scheduleMicrotask: queueMicrotask,
  getInstanceFromNode: () => null,
  beforeActiveInstanceBlur: () => {},
  afterActiveInstanceBlur: () => {},
  prepareScopeUpdate: () => {},
  getInstanceFromScope: () => null,
  detachDeletedInstance: () => {},
  appendChild: insert,
  appendChildToContainer: insert,
  insertBefore: insert,
  insertInContainerBefore: insert,
  removeChild: remove,
  removeChildFromContainer: remove,
  resetTextContent: instance => { (textChanges ?? createTextBatch()).push(instance.id, ""); },
  commitTextUpdate: (instance, _oldText, text) => {
    instance.props = { text };
    (textChanges ?? createTextBatch()).push(instance.id, text);
  },
  commitUpdate: update,
  hideInstance: instance => hide(instance, true),
  hideTextInstance: instance => hide(instance, true),
  unhideInstance: instance => hide(instance, false),
  unhideTextInstance: instance => hide(instance, false),
  clearContainer: container => {
    for (const child of [...container.children]) remove(container, child);
  },
  NotPendingTransition: null,
  // React's public Context type omits the fields consumed by its reconciler.
  HostTransitionContext: createContext(null) as unknown as Reconciler.ReactContext<null>,
  setCurrentUpdatePriority: value => { priority = value; },
  getCurrentUpdatePriority: () => priority,
  resolveUpdatePriority: () => priority,
  resetFormInstance: () => {},
  requestPostPaintCallback: callback => { setTimeout(() => callback(performance.now()), 0); },
  shouldAttemptEagerTransition: () => false,
  trackSchedulerEvent: () => {},
  resolveEventType: () => null,
  resolveEventTimeStamp: () => performance.now(),
  maySuspendCommit: () => false,
  preloadInstance: () => true,
  startSuspendingCommit: () => {},
  suspendInstance: () => {},
  waitForCommitToBeReady: () => null,
});

export function dispatchEvent(id: number, name: string, args: unknown[]) {
  const handler = mounted.get(id)?.props[name];
  if (typeof handler !== "function") return dispatchHostAction(id, name, args);
  const previous = priority;
  priority = DiscreteEventPriority;
  try {
    handler(...args);
  } finally {
    priority = previous;
  }
}

let developmentRoot: ReturnType<typeof reconciler.createContainer> | undefined;
if (process.env.NODE_ENV === "development") {
  reconciler.injectIntoDevTools({ bundleType: 1, version: "19.2.8", rendererPackageName: "ink" });
}

export function render(element: ReactNode) {
  if (process.env.NODE_ENV === "development" && developmentRoot) {
    const root = developmentRoot;
    reconciler.updateContainer(element, root, null, null);
    return () => reconciler.updateContainer(null, root, null, null);
  }
  onNativeMessage("event", message => {
    if (typeof message.id !== "number" || !Number.isSafeInteger(message.id)
      || typeof message.name !== "string" || !Array.isArray(message.args)) {
      throw new Error("Invalid native event");
    }
    dispatchEvent(message.id, message.name, message.args);
  });
  const container: Container = { id: 0, children: [] };
  const root = reconciler.createContainer(container, ConcurrentRoot, null, false, null, "", report, reportRecovered, reportRecovered, () => {});
  if (process.env.NODE_ENV === "development") developmentRoot = root;
  reconciler.updateContainer(element, root, null, null);
  return () => reconciler.updateContainer(null, root, null, null);
}
