declare const process: { env: { NODE_ENV: string } };
import { options, render as renderPreact, type Component, type ComponentChildren, type VNode } from "preact";
import type { ReactNode } from "react";
import { Activity, activityChanged, isComponentVisible } from "./visibility";
import { onNativeMessage } from "./native";
import { pressHandler } from "./press";
import type { Destination } from "./navigation";
import { listPatch, type NativeListItem } from "./list-patch";

declare const __inkCommit: (operations: Operation[]) => void;
declare const __inkPost: (message: string) => void;

type Props = Record<string, unknown>;
const hostProperties = Symbol("ink.hostProps");
type HostProps = Props & { [hostProperties]?: Props };
// Preact only needs tree links and mutation methods; no browser DOM is involved.
class HostNode {
  id: number;
  props: Props = {};
  parentNode: HostNode | null = null;
  firstChild: HostNode | null = null;
  lastChild: HostNode | null = null;
  previousSibling: HostNode | null = null;
  nextSibling: HostNode | null = null;
  hidden = false;
  mounted = false;
  readonly nodeType: number;
  readonly namespaceURI = "http://www.w3.org/1999/xhtml";
  constructor(readonly type: string, text?: string) {
    this.id = type === "#root" ? 0 : allocateHostId();
    this.nodeType = type === "#text" ? 3 : 1;
    if (text !== undefined) this.props = { text: String(text) };
  }
  get localName() { return this.type; }
  get childNodes(): HostNode[] {
    const nodes: HostNode[] = [];
    for (let child = this.firstChild; child; child = child.nextSibling) nodes.push(child);
    return nodes;
  }
  get data() { return this.props.text; }
  set data(text: unknown) {
    this.props = { text: String(text) };
    if (this.mounted) (textChanges ?? createTextBatch()).push(this.id, String(text));
  }
  // Native props and callbacks are read together from the completed host vnode.
  setAttribute() {}
  removeAttribute() {}
  addEventListener() {}
  removeEventListener() {}
  appendChild(child: HostNode) { this.insertBefore(child, null); }
  insertBefore(child: HostNode, before: HostNode | null) {
    if (child === before) return;
    if (child.parentNode) child.parentNode.unlink(child);
    child.parentNode = this;
    child.previousSibling = before ? before.previousSibling : this.lastChild;
    child.nextSibling = before;
    if (child.previousSibling) child.previousSibling.nextSibling = child;
    else this.firstChild = child;
    if (before) before.previousSibling = child;
    else this.lastChild = child;
    if (this.id === 0 || this.mounted) {
      publish(child);
      queue({ op: "insert", id: child.id, parent: this.id, before: before?.id ?? null });
    }
  }
  unlink(child: HostNode) {
    if (child.previousSibling) child.previousSibling.nextSibling = child.nextSibling;
    else this.firstChild = child.nextSibling;
    if (child.nextSibling) child.nextSibling.previousSibling = child.previousSibling;
    else this.lastChild = child.previousSibling;
    child.parentNode = child.previousSibling = child.nextSibling = null;
  }
  removeChild(child: HostNode) {
    this.unlink(child);
    if (child.mounted) {
      queue({ op: "remove", id: child.id, parent: this.id });
      forget(child);
    }
  }
}
type Instance = HostNode;
type Operation =
  | { op: "create"; id: number; type: string; props: Props }
  | { op: "update"; id: number; props: Props }
  | { op: "text"; changes: (number | string)[] }
  | { op: "insert"; id: number; parent: number; before: number | null }
  | { op: "remove"; id: number; parent: number }
  | { op: "hidden"; id: number; value: boolean };

const mounted = new Map<number, Instance>();
let nextId = 1;
let operations: Operation[] = [];
let textChanges: (number | string)[] | undefined;
let commitScheduled = false;

function allocateHostId() {
  if (!Number.isSafeInteger(nextId)) throw new RangeError("Host identifiers exhausted");
  return nextId++;
}

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
  const value = props.text ?? props.children;
  return typeof value === "string" ? value : typeof value === "number" || typeof value === "bigint" ? String(value) : null;
}

function hostProps(type: string, props: Props): Props {
  if (type !== "Text" || props.href === undefined && props.url === undefined) return props;
  const { href, url, onPress, ...rest } = props;
  return { ...rest, onPress: pressHandler({ href: href as Destination | undefined, url: url as string | undefined, onPress: onPress as (() => void) | undefined }) };
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
  if (instance.mounted) return;
  instance.mounted = true;
  mounted.set(instance.id, instance);
  queue({ op: "create", id: instance.id, type: instance.type, props: nativeProps(instance.props, instance.type) });
  if (instance.hidden) queue({ op: "hidden", id: instance.id, value: true });
  for (let child = instance.firstChild; child; child = child.nextSibling) {
    publish(child);
    queue({ op: "insert", id: child.id, parent: instance.id, before: null });
  }
}

function forget(instance: Instance) {
  instance.mounted = false;
  mounted.delete(instance.id);
  for (let child = instance.firstChild; child; child = child.nextSibling) forget(child);
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
  if (previous === props) return;
  const isText = type === "Text";
  if (isText) props = hostProps(type, props);
  instance.props = props;
  if (!instance.mounted) return;
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
    const children = props.text ?? props.children;
    if (children !== (previous.text ?? previous.children)) {
      if (typeof children === "string") return (textChanges ?? createTextBatch()).push(instance.id, children);
      if (typeof children === "number" || typeof children === "bigint") return (textChanges ?? createTextBatch()).push(instance.id, String(children));
      if (textContent(type, previous) !== null) (textChanges ?? createTextBatch()).push(instance.id, "");
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
  if (instance.hidden === value) return;
  instance.hidden = value;
  if (instance.mounted) queue({ op: "hidden", id: instance.id, value });
}

function errorMessage(error: unknown, info: { componentStack?: string | null }) {
  const message = error instanceof Error ? `${error.name}: ${error.message}\n${error.stack ?? ""}` : String(error);
  return info.componentStack ? `${message}\n${info.componentStack}` : message;
}

function report(error: unknown, info: { componentStack?: string | null }) {
  __inkPost(JSON.stringify({ type: "error", message: errorMessage(error, info) }));
}

// These fields are the stable, mangled option-hook ABI in the pinned Preact 10.
type HostVNode = VNode<Props> & { __e?: HostNode; __?: HostVNode; __k?: (HostVNode | null)[]; __c?: Component };
const hooks = options as typeof options & {
  __c?: (vnode: HostVNode, queue: unknown[]) => void;
  __inkNative?: boolean;
  __inkHost?: (node: HostNode, type: string, props: Props, vnode: HostVNode) => void;
};
hooks.__inkHost = (node, type, props, vnode) => {
  update(node, type, node.props, props);
  if (!node.parentNode && isHidden(vnode)) node.hidden = true;
};
const previousDiff = options.diffed;
options.diffed = vnode => {
  if (typeof vnode.type !== "string" && vnode.type != null) previousDiff?.(vnode);
  const host = vnode as HostVNode;
  if (!hooks.__inkNative) {
    if (typeof host.type === "string" && host.__e) update(host.__e, host.type, host.__e.props, (host.props as HostProps)[hostProperties]!);
    if (host.__e && (typeof host.type === "string" || host.type == null)) hide(host.__e, isHidden(host));
  }
  if (hooks.__inkNative && host.type == null && host.__e) hide(host.__e, isHidden(host));
  if (host.type === Activity && host.__c && activityChanged(host.__c)) visitVisibility(host);
};

function isHidden(vnode: HostVNode) {
  let component: Component | undefined;
  for (let parent = vnode.__; parent; parent = parent.__) {
    // A native ancestor already carries visibility for this entire branch.
    if (typeof parent.type === "string") return false;
    if (parent.__c) {
      component ??= parent.__c;
      if (parent.type === Activity) return !isComponentVisible(parent.__c);
    }
  }
  return component ? !isComponentVisible(component) : false;
}

function visitVisibility(vnode: HostVNode) {
  if (typeof vnode.type === "string" || vnode.type == null) {
    if (vnode.__e) hide(vnode.__e, isHidden(vnode));
    return;
  }
  for (const child of vnode.__k ?? []) if (child) visitVisibility(child);
}

const previousVNode = options.vnode;
options.vnode = vnode => {
  if (typeof vnode.type !== "string") {
    previousVNode?.(vnode);
    return;
  }
  const props = vnode.props as HostProps;
  // Preact reconciles children; Ink compares native props once, after the diff.
  // Keep the original props in this envelope so cloned compiler-cached vnodes
  // retain them without copying or normalising every native property.
  if (!hooks.__inkNative && !props[hostProperties]) {
    (vnode as VNode<HostProps>).props = {
      children: (textContent(vnode.type, props) === null ? props.children : undefined) as ComponentChildren,
      [hostProperties]: props,
    };
  }
};

// QuickJS has no browser paint cycle. Run effects before presenting queued ops.
options.requestAnimationFrame = callback => { queueMicrotask(callback); };
options.debounceRendering = queueMicrotask;
const previousCommit = hooks.__c;
hooks.__c = (vnode, components) => {
  previousCommit?.(vnode, components);
  if (commitScheduled) return;
  commitScheduled = true;
  queueMicrotask(() => queueMicrotask(() => {
    const committed = operations;
    operations = [];
    textChanges = undefined;
    commitScheduled = false;
    if (committed.length) __inkCommit(committed);
  }));
};

// Preact creates nodes through document, but all layout and painting stay native.
Object.assign(globalThis, { document: {
  createElementNS: (_namespace: string, type: string) => new HostNode(type),
  createElement: (type: string) => new HostNode(type),
  createTextNode: (text: string) => new HostNode("#text", text),
} });

export function dispatchEvent(id: number, name: string, args: unknown[]) {
  const handler = mounted.get(id)?.props[name];
  if (typeof handler === "function") handler(...args);
}

let developmentRoot: HostNode | undefined;
export function render(element: ReactNode) {
  const container = process.env.NODE_ENV === "development" && developmentRoot || new HostNode("#root");
  if (process.env.NODE_ENV === "development") developmentRoot = container;
  onNativeMessage("event", message => {
    if (typeof message.id !== "number" || !Number.isSafeInteger(message.id)
      || typeof message.name !== "string" || !Array.isArray(message.args)) {
      throw new Error("Invalid native event");
    }
    dispatchEvent(message.id, message.name, message.args);
  });
  const updateRoot = () => {
    try {
      renderPreact(element as ComponentChildren, container as unknown as Parameters<typeof renderPreact>[1]);
    } catch (error) {
      report(error, {});
    }
  };
  // Prefresh replaces mounted component types at the end of the bundle.
  // Apply the new root afterwards so matching components keep their state.
  if (process.env.NODE_ENV === "development") queueMicrotask(updateRoot);
  else updateRoot();
  return () => renderPreact(null, container as unknown as Parameters<typeof renderPreact>[1]);
}
