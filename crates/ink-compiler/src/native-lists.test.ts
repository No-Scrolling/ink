import ts from "typescript";
import { expect, test } from "bun:test";
import { compileNativeLists } from "./native-lists.js";

const imports = 'import { List, Text, Stack, Row } from "ink";';
const cases: [string, boolean][] = [
  ['<List renderItem={row => <Text>{row.name}</Text>} />', true],
  ['<List renderItem={row => <Row title={row.name} href={{path:"/item",params:{id:row.id}}}/>} />', true],
  ['<List renderItem={row => <Custom row={row}/>} />', false],
  ['<List renderItem={row => <Text>{format(row)}</Text>} />', false],
  ['<List renderItem={row => <Text ref={ref}>{row.name}</Text>} />', false],
  ['<List renderItem={row => <Text {...row}/>} />', false],
  ['<List renderItem={row => <Text>{row.name = "bad"}</Text>} />', false],
  ['<List renderItem={row => <Text>One &amp; two</Text>} />', false],
  ['<List renderItem={row => row.image ? <Row title={row.name}/> : <Text>{row.name}</Text>} />', false],
];

test.each(cases)("compilation eligibility: %s", (jsx, expected) => {
  const source = `${imports} function App() { return ${jsx}; }`;
  expect(compileNativeLists(ts, "case.tsx", source) !== source).toBe(expected);
});

test.each(["List", "Text"])("preserves shadowed %s imports", name => {
  const source = `${imports} function App(${name}) { return <List renderItem={row => <Text>{row.name}</Text>}/>; }`;
  expect(compileNativeLists(ts, "case.tsx", source)).toBe(source);
});

test("recognises import aliases", () => {
  const source = 'import {List as L,Text as T} from "ink"; const App = () => <L renderItem={x => <T>{x.name}</T>}/>';
  expect(compileNativeLists(ts, "alias.tsx", source)).not.toBe(source);
});
