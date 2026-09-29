// Observational diagnostic; actual renderer and current native QuickJS host.
import { createElement } from "../../../packages/ink/src/react";
import { render } from "../../../packages/ink/src/renderer";
import { List, Screen, Text } from "../../../packages/ink/src/index";
import { ReactList } from "../../../packages/ink/src/list";
import { nativeListRow, listText } from "../../../packages/ink/src/native-list";
declare const __inkPost: (message: string) => void;
declare const __inkIsProxy: (value: unknown) => boolean;
const commits: any[][] = [];
const originalCommit = (globalThis as any).__inkCommit;
(globalThis as any).__inkCommit = (batch: any[]) => { commits.push(batch); originalCommit(batch); };
const pause = () => new Promise(resolve => setTimeout(resolve, 25));
const template = JSON.stringify({ id: 0, type: "Stack", props: { gap: 0 }, children: [{ id: 1, type: "Text", props: { text: { $value: 0, path: ["values", 0] } }, children: [] }] });
const output: any[] = [];
async function inspect(kind: string, mode: string) {
  let labelReads = 0, reflectionTraps = 0, callTraps = 0;
  const handler = {
    get(target: any, name: PropertyKey, receiver: any) { if (name === "label") labelReads++; return Reflect.get(target, name, receiver); },
    getPrototypeOf(target: any) { reflectionTraps++; return Reflect.getPrototypeOf(target); },
    ownKeys(target: any) { reflectionTraps++; return Reflect.ownKeys(target); },
    getOwnPropertyDescriptor(target: any, key: PropertyKey) { reflectionTraps++; return Reflect.getOwnPropertyDescriptor(target, key); },
    apply(target: any, receiver: any, args: any[]) { callTraps++; return Reflect.apply(target, receiver, args); },
  };
  const scope: any = kind === "captured" ? new Proxy({ label: "Captured" }, handler) : kind === "function-getter"
    ? Object.defineProperty(() => {}, "label", { get() { labelReads++; return "Function property"; } }) : undefined;
  const callable = kind === "callable" ? new Proxy(() => "Callable", handler) : undefined;
  const items = Array.from({ length: 1000 }, (_, index) => {
    const item = { id: `row-${index}`, label: `Row ${index}` };
    return kind === "root" ? new Proxy(item, handler) : kind === "nested" ? { id: item.id, child: new Proxy(item, handler) } : kind === "callable" ? { ...item, value: callable } : item;
  });
  const callback = (item: any) => createElement(Text, null,
    kind === "nested" ? item.child.label : kind === "captured" || kind === "function-getter" ? scope!.label : kind === "callable" ? item.value() : item.label);
  const planned = nativeListRow(callback, (item: any) => [listText([
    kind === "nested" ? item.child.label : kind === "captured" || kind === "function-getter" ? scope!.label : kind === "callable" ? item.value() : item.label,
  ])], template, { identity: `audit-${kind}`, values: () => scope ? [scope] : [] });
  commits.length = 0;
  const stop = render(createElement(Screen, null, createElement(mode === "react" ? ReactList : List, {
    items, keyExtractor: (item: any) => item.id, renderItem: mode === "react" ? callback : planned,
  })));
  await pause();
  output.push({ kind, mode, dataset: items.length, labelReads, reflectionTraps, callTraps,
    nativeListNodes: commits.flat().filter(x => x.op === "create" && x.type === "NativeList").length });
  stop(); await pause();
}
(async () => {
  for (const kind of ["plain", "root", "nested", "captured", "callable", "function-getter"]) {
    for (const mode of ["react", "compiled"]) await inspect(kind, mode);
  }
  __inkPost(JSON.stringify({ type: "audit-final-proxy", nativeHelper: typeof __inkIsProxy === "function", results: output }));
})().catch(error => __inkPost(JSON.stringify({ type: "audit-final-error", message: String(error), stack: error.stack })));
