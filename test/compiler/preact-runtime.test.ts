import { expect, test } from "bun:test";
import { mkdtemp, mkdir, rm, symlink, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { createContext, runInContext } from "node:vm";

const repository = resolve(import.meta.dir, "../..");
type Operation = { op: string; id: number; type?: string; props?: Record<string, unknown>; changes?: (number | string)[]; value?: boolean };

async function project(run: (root: string, build: (source: string, development?: boolean) => Promise<string>) => Promise<void>) {
  await mkdir(resolve(repository, ".test-output"), { recursive: true });
  const root = await mkdtemp(resolve(repository, ".test-output/preact-contract-"));
  try {
    await mkdir(resolve(root, "node_modules"));
    await mkdir(resolve(root, ".ink"));
    await symlink(resolve(repository, "packages/ink"), resolve(root, "node_modules/ink"));
    await writeFile(resolve(root, "package.json"), '{"name":"preact-contract"}');
    await writeFile(resolve(root, "tsconfig.json"), '{"compilerOptions":{"jsx":"react-jsx"}}');
    await writeFile(resolve(root, ".ink/entry.tsx"), 'import { createElement } from "react"; import { render } from "ink/renderer"; import App from "../app"; render(createElement(App));');
    const build = async (source: string, development = false) => {
      await writeFile(resolve(root, "app.tsx"), source);
      const output = resolve(root, ".ink/app.js");
      const child = Bun.spawn([process.execPath, resolve(repository, "crates/ink-compiler/src/bundle-javascript.js"), root, resolve(root, ".ink/entry.tsx"), output, development ? "development" : "release"], { stdout: "pipe", stderr: "pipe" });
      const [code, stderr] = await Promise.all([child.exited, new Response(child.stderr).text()]);
      if (code) throw new Error(stderr);
      return Bun.file(output).text();
    };
    await run(root, build);
  } finally { await rm(root, { recursive: true, force: true }); }
}

function runtime() {
  const batches: Operation[][] = [];
  const messages: Record<string, unknown>[] = [];
  const context = createContext({
    queueMicrotask, setTimeout, clearTimeout, performance,
    __inkCommit: (batch: Operation[]) => batches.push(JSON.parse(JSON.stringify(batch))),
    __inkPost: (source: string) => messages.push(JSON.parse(source)),
  });
  return {
    batches, messages,
    evaluate: (source: string) => runInContext(source, context),
    send: (command: string, extra = {}) => runInContext(`__inkReceive(${JSON.stringify(JSON.stringify({ type: "contract", command, ...extra }))})`, context),
    settle: async () => { await new Promise(resolve => setTimeout(resolve, 0)); },
  };
}

const counter = (label: string, extraHook = "") => `
import { useEffect, useState } from "react";
import { Screen, Text } from "ink";
import { onNativeMessage } from "ink/native";
export default function App() {
  const [count, setCount] = useState(0);
  ${extraHook}
  useEffect(() => onNativeMessage("contract", () => setCount(value => value + 1)), []);
  return <Screen><Text>{${JSON.stringify(label)} + ": " + count}</Text></Screen>;
}`;

test("production JSX and compiler caches deliver current callbacks and batched text edits", async () => {
  await project(async (_root, build) => {
    const app = runtime();
    app.evaluate(await build(`
import React, { cloneElement, isValidElement, useState } from "react";
import { Screen, Stack, Text } from "ink";
export default function App() {
  const [count, setCount] = useState(0);
  const first = <Text>Unchanged source</Text>;
  if (!isValidElement(first) || !React.isValidElement(first)) throw new Error("Invalid Ink element");
  return <Screen><Stack>{cloneElement(first, {children: "First " + count})}<Text>{"Second " + count}</Text>
    <Text onPress={() => setCount(count + 1)}>Increase</Text><Text>Mixed {count}</Text></Stack></Screen>;
}`));
    await app.settle();
    const creates = app.batches.flat().filter(op => op.op === "create");
    expect(creates.map(op => [op.type, op.props?.text]).filter(([type]) => type === "Text")).toEqual([
      ["Text", "First 0"], ["Text", "Second 0"], ["Text", "Increase"], ["Text", undefined],
    ]);
    expect(creates.filter(op => op.type === "#text").map(op => op.props?.text)).toEqual(["Mixed ", "0"]);
    const mixedNumber = creates.find(op => op.type === "#text" && op.props?.text === "0")!.id;
    const first = creates.find(op => op.props?.text === "First 0")!.id;
    const second = creates.find(op => op.props?.text === "Second 0")!.id;
    const button = creates.find(op => op.props?.text === "Increase")!.id;
    app.batches.length = 0;
    for (let count = 1; count <= 2; count++) {
      app.evaluate(`__inkReceive(${JSON.stringify(JSON.stringify({ type: "event", id: button, name: "onPress", args: [] }))})`);
      await app.settle();
      expect(app.batches.splice(0).flat()).toEqual([{ op: "text", changes: [first, `First ${count}`, second, `Second ${count}`, mixedNumber, String(count)] }]);
    }
    expect(app.messages).toEqual([]);
  });
});

test("hidden screens retain state, disconnect Ink snapshots and reconnect with fresh data", async () => {
  await project(async (_root, build) => {
    const app = runtime();
    app.evaluate(await build(`
import { Activity, useEffect, useState } from "react";
import { Screen, Text, useSnapshot } from "ink";
import { onNativeMessage } from "ink/native";
let snapshot = { status: "ready", data: "Initial" };
const listeners = new Set();
const source = { getSnapshot: () => snapshot, subscribe(listener) {
  listeners.add(listener); __inkPost(JSON.stringify({type:"subscription", count:listeners.size}));
  return () => { listeners.delete(listener); __inkPost(JSON.stringify({type:"subscription", count:listeners.size})); };
}};
function Page() {
  const value = useSnapshot(source);
  const [count, setCount] = useState(0);
  return <Screen><Text>{value.data + " " + count}</Text><Text onPress={() => setCount(count + 1)}>Increase</Text></Screen>;
}
export default function App() {
  const [visible, setVisible] = useState(true);
  useEffect(() => onNativeMessage("contract", message => {
    if (message.command === "hide") setVisible(false);
    if (message.command === "show") setVisible(true);
    if (message.command === "publish") { snapshot = {status:"ready", data:"Fresh"}; for (const listener of listeners) listener(); }
  }), []);
  return <Activity mode={visible ? "visible" : "hidden"}><Page /></Activity>;
}`));
    await app.settle();
    const creates = app.batches.flat().filter(op => op.op === "create");
    const screen = creates.find(op => op.type === "Screen")!.id;
    const button = creates.find(op => op.props?.text === "Increase")!.id;
    app.evaluate(`__inkReceive(${JSON.stringify(JSON.stringify({ type: "event", id: button, name: "onPress", args: [] }))})`);
    await app.settle();
    expect(app.batches.flat().some(op => op.changes?.includes("Initial 1"))).toBe(true);
    app.batches.length = 0;
    app.send("hide"); await app.settle();
    expect(app.messages.filter(message => message.type === "subscription").map(message => message.count)).toEqual([1, 0]);
    expect(app.batches.flat().some(op => op.op === "hidden" && op.id === screen && op.value === true)).toBe(true);
    app.batches.length = 0;
    app.send("publish"); await app.settle();
    expect(app.batches).toEqual([]);
    app.send("show"); await app.settle();
    expect(app.messages.filter(message => message.type === "subscription").map(message => message.count)).toEqual([1, 0, 1]);
    expect(app.batches.flat().some(op => op.changes?.includes("Fresh 1"))).toBe(true);
    expect(app.batches.flat().some(op => op.op === "create" || op.op === "remove")).toBe(false);
  });
});

test("fast refresh preserves state for body edits and resets it when the hook signature changes", async () => {
  await project(async (_root, build) => {
    const app = runtime();
    app.evaluate(await build(counter("Before"), true));
    await app.settle();
    app.send("increase"); await app.settle();
    expect(app.batches.flat().some(op => op.props?.text === "Before: 1" || op.changes?.includes("Before: 1"))).toBe(true);
    app.batches.length = 0;
    app.evaluate(await build(counter("After"), true));
    await app.settle();
    expect(app.batches.flat().some(op => op.props?.text === "After: 1" || op.changes?.includes("After: 1"))).toBe(true);
    expect(app.batches.flat().some(op => op.op === "create" || op.op === "remove")).toBe(false);
    app.batches.length = 0;
    app.evaluate(await build(counter("Reset", "const [other] = useState(0);"), true));
    await app.settle();
    expect(app.batches.flat().some(op => op.props?.text === "Reset: 0" || op.changes?.includes("Reset: 0"))).toBe(true);
    expect(app.messages.filter(message => message.type === "error")).toEqual([]);
  });
}, 30_000);

test("Suspense replaces its fallback after a lazy component resolves", async () => {
  await project(async (_root, build) => {
    const app = runtime();
    app.evaluate(await build(`
import React, { lazy, Suspense, useEffect } from "react";
import { Screen, Text } from "ink";
import { onNativeMessage } from "ink/native";
let resolvePage;
const Page = lazy(() => new Promise(resolve => { resolvePage = resolve; }));
export default function App() {
  useEffect(() => onNativeMessage("contract", () => resolvePage({default: () => <Text>Ready</Text>})), []);
  return <Screen><Suspense fallback={<Text>Loading</Text>}><Page /></Suspense></Screen>;
}`));
    await app.settle();
    expect(app.batches.flat().some(op => op.props?.text === "Loading")).toBe(true);
    app.batches.length = 0;
    app.send("resolve"); await app.settle();
    expect(app.batches.flat().some(op => op.props?.text === "Ready")).toBe(true);
    expect(app.batches.flat().some(op => op.op === "remove")).toBe(true);
    expect(app.messages.filter(message => message.type === "error")).toEqual([]);
  });
});

const activityContract = `import { useLibraryEffects } from "effect-library";
import { Activity, memo, useEffect, useEffectEvent, useLayoutEffect, useState, useSyncExternalStore } from "react";
import { onNativeMessage } from "ink/native";
const log = label => __inkPost(JSON.stringify({type:"effect", label}));
let snapshot = 0;
let bump, read;
const listeners = new Set();
const subscribe = listener => { log("subscribe"); listeners.add(listener); return () => {log("unsubscribe");listeners.delete(listener);}; };
const getSnapshot = () => snapshot;
const Child = memo(function Child({value}) {
 useLibraryEffects("child", value);
 const data = useSyncExternalStore(subscribe, getSnapshot);
 return <Text>{"Data "+data}</Text>;
});
function Parent({value}) {
 useLibraryEffects("parent", value);
 const [count,setCount] = useState(0);
 bump = () => setCount(value => value + 1);
 read = useEffectEvent(() => __inkPost(JSON.stringify({type:"callback",count})));
 useLayoutEffect(() => { read(); return () => { read(); }; }, [count]);
 return <Stack><Text onPress={()=>setCount(count+1)}>{"Count "+count}</Text><Child value={value}/></Stack>;
}
export default function App() {
 const [visible,setVisible] = useState(true);
 const [value,setValue] = useState(0);
 const [mounted,setMounted] = useState(true);
 useEffect(() => onNativeMessage("contract", message => {
  if(message.command==="hide")setVisible(false);
  if(message.command==="show")setVisible(true);
  if(message.command==="change")setValue(1);
  if(message.command==="bump")bump();
  if(message.command==="read")read();
  if(message.command==="publish"){snapshot=2;for(const listener of listeners)listener();}
  if(message.command==="remove")setMounted(false);
 }),[]);
 return mounted && <Activity mode={visible?"visible":"hidden"}><Parent value={value}/></Activity>;
}
const Stack = "Stack", Text = "Text";
`;

// Expected ordering was checked against current main's real React renderer.
// The assertions describe the contract rather than comparing two runtimes.
test("Activity disconnects app and library effects in React order, retaining state and native nodes", async () => {
  await project(async (root, build) => {
    const library = resolve(root, "node_modules/effect-library");
    await mkdir(library);
    await writeFile(resolve(library, "package.json"), '{"name":"effect-library","main":"index.js"}');
    await writeFile(resolve(library, "index.js"), `import React from "react";
const log = label => __inkPost(JSON.stringify({type:"effect", label}));
export function useLibraryEffects(name, value) {
 React.useLayoutEffect(() => { log(name+" layout "+value); return () => log(name+" layout cleanup "+value); }, [value]);
 React.useEffect(() => { log(name+" passive "+value); return () => log(name+" passive cleanup "+value); }, [value]);
}
`);
    const app = runtime();
    const labels = () => app.messages.splice(0).filter(message => message.type === "effect").map(message => message.label);
    app.evaluate(await build(activityContract));
    await app.settle();
    expect(labels()).toEqual(["child layout 0", "parent layout 0", "child passive 0", "subscribe", "parent passive 0"]);
    const button = app.batches.flat().find(op => op.props?.text === "Count 0")!.id;
    app.evaluate(`__inkReceive(${JSON.stringify(JSON.stringify({type:"event",id:button,name:"onPress",args:[]}))})`);
    await app.settle();
    expect(app.messages.filter(message => message.type === "callback")).toEqual([{type:"callback",count:1},{type:"callback",count:1}]);
    expect(labels()).toEqual([]);
    app.batches.length = 0;
    app.send("hide"); await app.settle();
    expect(labels()).toEqual(["parent layout cleanup 0", "child layout cleanup 0", "parent passive cleanup 0", "child passive cleanup 0", "unsubscribe"]);
    app.batches.length = 0;
    app.send("change"); await app.settle();
    app.send("bump"); await app.settle();
    expect(app.messages.filter(message => message.type === "callback")).toEqual([]);
    app.send("read"); await app.settle();
    expect(app.messages).toContainEqual({type:"callback",count:2});
    app.send("publish"); await app.settle();
    expect(labels()).toEqual([]);
    app.send("show"); await app.settle();
    expect(labels()).toEqual(["child layout 1", "parent layout 1", "child passive 1", "subscribe", "parent passive 1"]);
    expect(app.batches.flat().some(op => op.op === "create" || op.op === "remove")).toBe(false);
    expect(app.batches.flat().some(op => op.changes?.includes("Count 0"))).toBe(false);
    expect(app.batches.flat().some(op => op.changes?.includes("Data 2"))).toBe(true);
    expect(app.batches.flat().some(op => op.changes?.includes("Count 2"))).toBe(true);
    app.send("remove"); await app.settle();
    expect(labels()).toEqual(["parent layout cleanup 1", "child layout cleanup 1", "parent passive cleanup 1", "child passive cleanup 1", "unsubscribe"]);
  });
});

test("nested Activity keeps initially hidden effects disconnected and respects hidden ancestors", async () => {
  await project(async (_root, build) => {
    const app = runtime();
    app.evaluate(await build(`
import { Activity, memo, useEffect, useLayoutEffect, useState } from "react";
import { onNativeMessage } from "ink/native";
const log = label => __inkPost(JSON.stringify({type:"effect", label}));
const Child = memo(function Child() {
  useLayoutEffect(() => {log("child layout");return () => log("child layout cleanup");}, []);
  useEffect(() => {log("child passive");return () => log("child passive cleanup");}, []);
  return "Retained child";
});
function Parent({inner}) {
  useLayoutEffect(() => {log("parent layout");return () => log("parent layout cleanup");}, []);
  useEffect(() => {log("parent passive");return () => log("parent passive cleanup");}, []);
  return <Activity mode={inner ? "visible" : "hidden"}><Child /></Activity>;
}
export default function App() {
  const [outer,setOuter] = useState(false);
  const [inner,setInner] = useState(false);
  useEffect(() => onNativeMessage("contract", message => {
    if(message.command==="outerShow")setOuter(true);
    if(message.command==="outerHide")setOuter(false);
    if(message.command==="innerShow")setInner(true);
    if(message.command==="innerHide")setInner(false);
  }),[]);
  return <Activity mode={outer ? "visible" : "hidden"}><Parent inner={inner}/></Activity>;
}`));
    const labels = () => app.messages.splice(0).filter(message => message.type === "effect").map(message => message.label);
    await app.settle();
    expect(labels()).toEqual([]);
    app.send("innerShow"); await app.settle();
    expect(labels()).toEqual([]);
    app.send("outerShow"); await app.settle();
    expect(labels()).toEqual(["child layout", "parent layout", "child passive", "parent passive"]);
    app.send("outerHide"); await app.settle();
    expect(labels()).toEqual(["parent layout cleanup", "child layout cleanup", "parent passive cleanup", "child passive cleanup"]);
    app.send("innerHide"); await app.settle();
    expect(labels()).toEqual([]);
    app.send("outerShow"); await app.settle();
    expect(labels()).toEqual(["parent layout", "parent passive"]);
    app.send("innerShow"); await app.settle();
    expect(labels()).toEqual(["child layout", "child passive"]);
    expect(app.messages.filter(message => message.type === "error")).toEqual([]);
  });
});

test("Activity disconnects imperative handles and reconnects retained component state", async () => {
  await project(async (_root, build) => {
    const app = runtime();
    app.evaluate(await build(`
import { Activity, forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import { onNativeMessage } from "ink/native";
const Page = forwardRef(function Page(props, ref) {
  const [value] = useState(41);
  useImperativeHandle(ref, () => ({value}), []);
  return null;
});
export default function App() {
  const [visible,setVisible] = useState(true);
  const ref = useRef(null);
  useEffect(() => onNativeMessage("contract", message => {
    if(message.command==="hide")setVisible(false);
    if(message.command==="show")setVisible(true);
    if(message.command==="read")__inkPost(JSON.stringify({type:"handle",value:ref.current?.value ?? null}));
  }),[]);
  return <Activity mode={visible ? "visible" : "hidden"}><Page ref={ref}/></Activity>;
}`));
    await app.settle();
    app.send("read"); await app.settle();
    app.send("hide"); await app.settle();
    app.send("read"); await app.settle();
    app.send("show"); await app.settle();
    app.send("read"); await app.settle();
    expect(app.messages).toEqual([{type:"handle",value:41},{type:"handle",value:null},{type:"handle",value:41}]);
  });
});

test("cached host children retain context and route effect failures to their error boundary", async () => {
  await project(async (_root, build) => {
    const app = runtime();
    app.evaluate(await build(`
import React, { createContext, useContext, useEffect, useState } from "react";
import { Stack, Text } from "ink";
import { onNativeMessage } from "ink/native";
const Context = createContext(0);
class Boundary extends React.Component {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  render() { return this.state.failed ? <Text>Recovered effect</Text> : this.props.children; }
}
function Page() {
  const value = useContext(Context);
  useEffect(() => { if (value === 1) throw new Error("effect failed"); }, [value]);
  return <Text>Before effect</Text>;
}
const retained = <Stack><Page /></Stack>;
export default function App() {
  const [value,setValue] = useState(0);
  useEffect(() => onNativeMessage("contract", () => setValue(1)), []);
  return <Boundary><Context.Provider value={value}>{retained}</Context.Provider></Boundary>;
}
`));
    await app.settle();
    expect(app.batches.flat().some(op => op.props?.text === "Before effect")).toBe(true);
    app.send("fail"); await app.settle();
    expect(app.batches.flat().some(op => op.props?.text === "Recovered effect")).toBe(true);
    expect(app.messages).toEqual([]);
  });
});

test("Suspense does not start effects from a render that suspended", async () => {
  await project(async (_root, build) => {
    const app = runtime();
    app.evaluate(await build(`
import { Suspense, useEffect } from "react";
import { Text } from "ink";
import { onNativeMessage } from "ink/native";
let ready = false, resolve;
const pending = new Promise(done => { resolve = done; });
function Page() {
  useEffect(() => { __inkPost(JSON.stringify({type:"effect",label:"started"})); }, []);
  if (!ready) throw pending;
  return <Text>Ready content</Text>;
}
export default function App() {
  useEffect(() => onNativeMessage("contract", () => { ready = true; resolve(); }), []);
  return <Suspense fallback={<Text>Loading content</Text>}><Page /></Suspense>;
}
`));
    await app.settle();
    expect(app.messages).toEqual([]);
    expect(app.batches.flat().some(op => op.props?.text === "Loading content")).toBe(true);
    app.send("resolve"); await app.settle();
    expect(app.messages).toEqual([{type:"effect",label:"started"}]);
    expect(app.batches.flat().some(op => op.props?.text === "Ready content")).toBe(true);
  });
});

test("15-second transport controls select their filled icons and seek in milliseconds", async () => {
  await project(async (_root, build) => {
    const app = runtime();
    app.evaluate(await build(`
import { PlayingScreen } from "ink";
export default function App() {
  return <PlayingScreen title="Seek contract" artists={[]}
    previous={{kind: "skip", seconds: 15}} next={{kind: "skip", seconds: 15}}
    playback={{state: {position: 42000, duration: 213000, playWhenReady: false},
      toggle() {}, seek() {}, seekBy(offset) { __inkPost(JSON.stringify({offset})); }}} />;
}`));
    await app.settle();
    const screen = app.batches.flat().find(op => op.type === "PlayingScreen")!;
    expect(screen.props).toMatchObject({
      previousIcon: "filled:replay_15", nextIcon: "filled:forward_15",
      previousDisabled: false, nextDisabled: false,
    });
    for (const name of ["onPreviousPress", "onNextPress"]) {
      app.evaluate(`__inkReceive(${JSON.stringify(JSON.stringify({ type: "event", id: screen.id, name, args: [] }))})`);
      await app.settle();
    }
    expect(app.messages.filter(message => "offset" in message)).toEqual([{ offset: -15000 }, { offset: 15000 }]);
  });
});
