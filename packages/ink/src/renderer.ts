declare const process: { env: { NODE_ENV: string } };
import { createContext, type ReactNode } from "react";
import Reconciler from "react-reconciler";
import { ConcurrentRoot, DefaultEventPriority, DiscreteEventPriority } from "react-reconciler/constants";
import { onNativeMessage } from "./native";
import { openLink } from "./external";

declare const __inkPost: (message: string) => void;

type Props = Record<string, unknown>;
type Instance = { id: number; type: string; props: Props; children: Instance[] };
type Container = { id: number; children: Instance[] };
type Operation =
  | { op: "create"; id: number; type: string; props: Props }
  | { op: "update"; id: number; props: Props }
  | { op: "text"; ids: number[]; values: string[] }
  | { op: "insert"; id: number; parent: number; before: number | null }
  | { op: "remove"; id: number; parent: number }
  | { op: "hidden"; id: number; value: boolean };

const mounted = new Map<number, Instance>();
let operations: Operation[] = [];
let commitScheduled = false;
let nextId = 1;
let priority = DefaultEventPriority;
const hostContext = {};

function queueText(id: number, text: string) {
  let batch = operations[operations.length - 1];
  if (batch?.op !== "text") {
    batch = { op: "text", ids: [], values: [] };
    operations.push(batch);
  }
  batch.ids.push(id);
  batch.values.push(text);
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
  for (const [name, value] of Object.entries(props)) {
    if (name === "children" || name === "ref" || value === undefined) continue;
    result[name] = typeof value === "function" ? true : value;
  }
  const text = textContent(type, props);
  if (text !== null) result.text = text;
  return result;
}

function publish(instance: Instance) {
  if (mounted.has(instance.id)) return;
  mounted.set(instance.id, instance);
  operations.push({ op: "create", id: instance.id, type: instance.type, props: nativeProps(instance.props, instance.type) });
  for (const child of instance.children) {
    publish(child);
    operations.push({ op: "insert", id: child.id, parent: instance.id, before: null });
  }
}

function insert(parent: Container, child: Instance, before?: Instance) {
  const previous = parent.children.indexOf(child);
  if (previous !== -1) parent.children.splice(previous, 1);
  const index = before ? parent.children.indexOf(before) : parent.children.length;
  parent.children.splice(index, 0, child);
  publish(child);
  operations.push({ op: "insert", id: child.id, parent: parent.id, before: before?.id ?? null });
}

function forget(instance: Instance) {
  mounted.delete(instance.id);
  for (const child of instance.children) forget(child);
}

function remove(parent: Container, child: Instance) {
  parent.children.splice(parent.children.indexOf(child), 1);
  operations.push({ op: "remove", id: child.id, parent: parent.id });
  forget(child);
}

function update(instance: Instance, _type: string, _oldProps: Props, props: Props) {
  const previous = instance.props;
  if (instance.type === "Text" && props.href !== undefined) props = hostProps(instance.type, props);
  instance.props = props;
  if (instance.type === "Text") {
    const children = props.children;
    if (children !== previous.children) {
      if (typeof children === "string") queueText(instance.id, children);
      else if (typeof children === "number" || typeof children === "bigint") queueText(instance.id, String(children));
    }
    if (previous.size === props.size && previous.width === props.width
      && previous.align === props.align && previous.maxLines === props.maxLines
      && previous.tabularNumbers === props.tabularNumbers
      && (previous.onPress === props.onPress || nativeValue(previous.onPress) === nativeValue(props.onPress))) return;
    operations.push({ op: "update", id: instance.id, props: nativeProps(props, instance.type) });
    return;
  }
  let previousCount = 0;
  let changed = false;
  for (const name of Object.keys(previous)) {
    const value = previous[name];
    if (name === "children" || name === "ref" || value === undefined) continue;
    previousCount++;
    const next = props[name];
    if (!Object.hasOwn(props, name) || !Object.is(
      typeof value === "function" ? true : value,
      typeof next === "function" ? true : next,
    )) {
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
  const next = nativeProps(props, instance.type);
  if (instance.type === "List" && nativeValue(previous.revision) === next.revision
    && nativeValue(previous.keys) === next.keys && nativeValue(previous.contentVersions) === next.contentVersions) {
    delete next.keys;
    delete next.contentVersions;
  }
  operations.push({ op: "update", id: instance.id, props: next });
}

function hide(instance: Instance, value: boolean) {
  operations.push({ op: "hidden", id: instance.id, value });
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
  createInstance: (type, props) => ({ id: nextId++, type, props: hostProps(type, props), children: [] }),
  createTextInstance: text => ({ id: nextId++, type: "#text", props: { text }, children: [] }),
  appendInitialChild: (parent, child) => { parent.children.push(child); },
  finalizeInitialChildren: () => false,
  shouldSetTextContent: (type, props) => {
    if (type === "Text" && props.href !== undefined && props.onPress) {
      throw new Error("Text accepts either href or onPress");
    }
    const kind = typeof props.children;
    return type === "Text" && (kind === "string" || kind === "number" || kind === "bigint");
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
      operations = [];
      commitScheduled = false;
      if (committed.length) __inkPost(JSON.stringify({ type: "commit", operations: committed }));
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
  resetTextContent: instance => { queueText(instance.id, ""); },
  commitTextUpdate: (instance, _oldText, text) => {
    instance.props = { text };
    queueText(instance.id, text);
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
  if (typeof handler !== "function") return;
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
