// Observational control: real QuickJS and renderer; no repository tests added.
import { createElement } from "../../../packages/ink/src/react";
import { render } from "../../../packages/ink/src/renderer";
import { List, Screen, Text } from "../../../packages/ink/src/index";
import { ReactList } from "../../../packages/ink/src/list";
import { nativeListRow, listText } from "../../../packages/ink/src/native-list";
declare const __inkPost: (message: string) => void;
const errors: string[] = [], commits: any[][] = [];
const originalPost = (globalThis as any).__inkPost;
(globalThis as any).__inkPost = (message: string) => {
  const value = JSON.parse(message);
  if (value.type === "error") errors.push(value.message);
  else originalPost(message);
};
const originalCommit = (globalThis as any).__inkCommit;
(globalThis as any).__inkCommit = (batch: any[]) => { commits.push(batch); originalCommit(batch); };
const pause = () => new Promise(resolve => setTimeout(resolve, 25));
const template = JSON.stringify({ id: 0, type: "Stack", props: { gap: 0 }, children: [{ id: 1, type: "Text", props: { text: { $value: 0, path: ["values", 0] } }, children: [] }] });
const output: any[] = [];
async function inspect(depth: number, shared: boolean, mode: string) {
  let graph: any = { label: "Leaf" };
  for (let step = 0; step < depth; step++) graph = shared ? { left: graph, right: graph } : { left: graph };
  const value = () => { let current = graph; for (let step = 0; step < depth; step++) current = current.left; return current.label; };
  const items = Array.from({ length: 100 }, (_, index) => ({ id: `depth-${index}` }));
  let reads = 0;
  const callback = () => { reads++; return createElement(Text, null, value()); };
  const planned = nativeListRow(callback, () => { reads++; return [listText([value()])]; }, template,
    { identity: `graph-${depth}`, values: () => [graph] });
  errors.length = 0; commits.length = 0;
  const stop = render(createElement(Screen, null, createElement(mode === "react" ? ReactList : List, {
    items, keyExtractor: (item: any) => item.id, renderItem: mode === "react" ? callback : planned,
  })));
  await pause();
  output.push({ depth, distinctObjects: depth + 1, shared, mode, reads, errors: [...errors],
    nativeListNodes: commits.flat().filter(x => x.op === "create" && x.type === "NativeList").length });
  stop(); await pause();
}
(async () => {
  for (const depth of [4, 8, 12, 16, 20, 24, 64]) {
    for (const shared of [false, true]) for (const mode of ["react", "compiled"]) await inspect(depth, shared, mode);
  }
  __inkPost(JSON.stringify({ type: "audit-final-proxy", diagnostic: "capture-depth", results: output }));
})().catch(error => originalPost(JSON.stringify({ type: "audit-final-error", message: String(error), stack: error.stack })));
