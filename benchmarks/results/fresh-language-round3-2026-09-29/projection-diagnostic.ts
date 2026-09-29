// Observational diagnostic of the current public renderer and list paths.
// This writes no application state and is not a repository test.
let identifier = 1;
const commits: any[][] = [];
const errors: string[] = [];
Object.assign(globalThis, {
  __inkNextId: () => identifier++,
  __inkCommit: (batch: any[]) => commits.push(batch),
  __inkPost: (message: string) => { const value = JSON.parse(message); if (value.type === "error") errors.push(value.message); },
});
const { createElement } = await import("../../../packages/ink/src/react.ts");
const { render } = await import("../../../packages/ink/src/renderer.ts");
const { List, Screen, Text } = await import("../../../packages/ink/src/index.ts");
const { nativeListRow, listText } = await import("../../../packages/ink/src/native-list.ts");
const { ReactList } = await import("../../../packages/ink/src/list.ts");
const { compileNativeLists } = await import("../../../crates/ink-compiler/src/native-lists.js");
const ts = (await import("typescript")).default;
const example = 'import { List, Text } from "ink"; const App = () => <List items={items} keyExtractor={x=>x.id} renderItem={x=><Text>{x.label}</Text>}/>;';
const transformed = compileNativeLists(ts, "proxy-case.tsx", example);
await Bun.write(new URL("compiler-proxy-case.txt", import.meta.url), transformed);
const template = JSON.stringify({ id: 0, type: "Stack", props: { gap: 0 }, children: [{ id: 1, type: "Text", props: { text: { $value: 0, path: ["values", 0] } }, children: [] }] });
const pause = () => new Promise(resolve => setTimeout(resolve, 25));
const output: any[] = [];
for (const mode of ["react", "compiled"]) {
  let reads = 0;
  const items = Array.from({ length: 1000 }, (_, index) => new Proxy({ id: `row-${index}`, label: `Row ${index}` }, {
    get(target, name, receiver) { if (name === "label") reads++; return Reflect.get(target, name, receiver); },
  }));
  const callback = (item: any) => createElement(Text, null, item.label);
  const planned = nativeListRow(callback, (item: any) => [listText([item.label])], template, { identity: "proxy-case", values: () => [] });
  commits.length = 0; errors.length = 0;
  const stop = render(createElement(Screen, null, createElement(mode === "react" ? ReactList : List, {
    items, keyExtractor: (item: any) => item.id, renderItem: mode === "react" ? callback : planned,
  })));
  await pause();
  output.push({ mode, dataset: items.length, labelReads: reads, nativeListNodes: commits.flat().filter(x => x.op === "create" && x.type === "NativeList").length, errors: [...errors] });
  stop(); await pause();
}
await Bun.write(new URL("projection-diagnostic.json", import.meta.url), JSON.stringify({ compilerEligible: transformed !== example, results: output }, null, 2) + "\n");
console.log(JSON.stringify({ compilerEligible: transformed !== example, results: output }));
