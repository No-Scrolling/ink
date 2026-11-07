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
import { useState } from "react";
import { Screen, Stack, Text } from "ink";
export default function App() {
  const [count, setCount] = useState(0);
  return <Screen><Stack><Text>{"First " + count}</Text><Text>{"Second " + count}</Text>
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
