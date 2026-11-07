import { Fragment, options, type ComponentType, type VNode } from "preact";
export { Fragment };
let nextId = 0;

/** Preact's vnode contract without unused browser/debug fields on every node. */
export function jsx(type: string | ComponentType, incoming: Record<string, unknown> | null, key?: string) {
  let props = incoming ?? {};
  let ref: unknown;
  if ("ref" in props) {
    ({ ref, ...props } = props);
  }
  if (typeof type === "function" && type.defaultProps) {
    for (const name in type.defaultProps) {
      if (props[name] === undefined) props[name] = (type.defaultProps as Record<string, unknown>)[name];
    }
  }
  const vnode = { type, props, key, ref, constructor: undefined, __v: --nextId } as unknown as VNode;
  if (typeof type === "function") options.vnode?.(vnode);
  return vnode;
}
export { jsx as jsxs, jsx as jsxDEV };
