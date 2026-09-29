// Observational diagnostic; actual renderer and current native QuickJS host.
import { createElement, useState } from "../../../packages/ink/src/react";
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
  const scope = kind === "captured" ? new Proxy({ label: "Captured" }, handler) : undefined;
  const callable = kind === "callable" ? new Proxy(() => "Callable", handler) : undefined;
  const items = Array.from({ length: 1000 }, (_, index) => {
    const item = { id: `row-${index}`, label: `Row ${index}` };
    return kind === "root" ? new Proxy(item, handler) : kind === "nested" ? { id: item.id, child: new Proxy(item, handler) } : kind === "callable" ? { ...item, value: callable } : item;
  });
  const callback = (item: any) => createElement(Text, null,
    kind === "nested" ? item.child.label : kind === "captured" ? scope!.label : kind === "callable" ? item.value() : item.label);
  const planned = nativeListRow(callback, (item: any) => [listText([
    kind === "nested" ? item.child.label : kind === "captured" ? scope!.label : kind === "callable" ? item.value() : item.label,
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
  for (const kind of ["plain", "root", "nested", "captured", "callable"]) {
    for (const mode of ["react", "compiled"]) await inspect(kind, mode);
  }
  // Acyclic shared capture graph: 25 distinct objects, 2^25 unfolded leaves.
  const leaf = { label: "Before" };
  let graph: any = leaf;
  for (let depth = 0; depth < 24; depth++) graph = { left: graph, right: graph };
  const items = Array.from({ length: 1000 }, (_, index) => ({ id: `shared-${index}` }));
  const keyExtractor = (item: any) => item.id;
  let projections = 0, refresh: () => void;
  const callback = () => createElement(Text, null, leaf.label);
  function App() {
    const [, setTick] = useState(0);
    refresh = () => setTick(value => value + 1);
    const planned = nativeListRow(callback, () => { projections++; return [listText([leaf.label])]; }, template,
      { identity: "shared-graph", values: () => [graph, graph] });
    return createElement(Screen, null, createElement(List, { items, keyExtractor, renderItem: planned }));
  }
  commits.length = 0;
  const stop = render(createElement(App));
  await pause();
  const initialProjections = projections;
  refresh!(); await pause();
  const stableProjections = projections - initialProjections;
  leaf.label = "After";
  refresh!(); await pause();
  const changedProjections = projections - initialProjections - stableProjections;
  const graphEvidence = { distinctObjects: 25, repeatedScopeEntries: 2, initialProjections, stableProjections, changedProjections,
    nativeListNodes: commits.flat().filter(x => x.op === "create" && x.type === "NativeList").length,
    updates: commits.flat().filter(x => x.op === "update") };
  stop(); await pause();
  __inkPost(JSON.stringify({ type: "audit-final-proxy", nativeHelper: typeof __inkIsProxy === "function", results: output, captureGraph: graphEvidence }));
})().catch(error => __inkPost(JSON.stringify({ type: "audit-final-error", message: String(error), stack: error.stack })));
