import ts from "typescript";
import { expect, test } from "bun:test";
import { compileNativeLists } from "../../crates/ink-compiler/src/native-lists.js";
import { nativeListPlan } from "../../packages/ink/src/native-list";
import { loadCompiled } from "./helpers";

const imports = 'import { List, Text, Stack, Row } from "ink";';
const fallback = [
  '<List renderItem={row => <Custom row={row}/>} />',
  '<List renderItem={row => <Text>{format(row)}</Text>} />',
  '<List renderItem={row => <Text ref={ref}>{row.name}</Text>} />',
  '<List renderItem={row => <Text {...row}/>} />',
  '<List renderItem={row => <Text>{row.name = "bad"}</Text>} />',
  '<List renderItem={row => <Text>One &amp; two</Text>} />',
  '<List renderItem={row => row.image ? <Row title={row.name}/> : <Text>{row.name}</Text>} />',
];

test.each(fallback)("preserves React-only rendering: %s", jsx => {
  const source = `${imports} function App() { return ${jsx}; }`;
  expect(compileNativeLists(ts, "case.tsx", source)).toBe(source);
});

test.each(["List", "Text"])("preserves shadowed %s imports", name => {
  const source = `${imports} function App(${name}) { return <List renderItem={row => <Text>{row.name}</Text>}/>; }`;
  expect(compileNativeLists(ts, "case.tsx", source)).toBe(source);
});

test("recognises aliases by binding and produces an executable text plan", async () => {
  const { module } = await loadCompiled('import {List as L,Text as T} from "ink"; export const row = <L renderItem={x => <T>{x.name}</T>}/>;');
  const plan = nativeListPlan(module.row.props.renderItem)!;
  expect(plan.project({ name: "Aliased" }, 0)).toEqual(["Aliased"]);
  expect(JSON.parse(plan.template).children[0].type).toBe("Text");
});

test("the original eligible href Row lowers into a deferred navigation action", async () => {
  const { module } = await loadCompiled('import {List,Row} from "ink"; export const row = <List renderItem={item => <Row title={item.name} href={{path:"/item",params:{id:item.id}}}/>}/>;');
  const plan = nativeListPlan(module.row.props.renderItem)!;
  const fields = plan.project({ id: "42", name: "Linked" }, 0)[0] as { title: string; onPress: () => void };
  expect(fields.title).toBe("Linked");
  expect(typeof fields.onPress).toBe("function");
  expect(() => fields.onPress()).toThrow("Navigation is unavailable before the app mounts");
  expect(JSON.parse(plan.template).children[0].props.onPress).toEqual({ $value: 0, path: ["values", 0, "onPress"] });
});
