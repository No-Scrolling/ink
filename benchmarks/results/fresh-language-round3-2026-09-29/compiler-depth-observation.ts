import ts from "typescript";
import { compileNativeLists } from "../../../crates/ink-compiler/src/native-lists.js";
const cases = [12, 16, 20, 24, 64].map(depth => {
  const source = `import { List, Text } from "ink"; const App = () => <List items={items} keyExtractor={item => item.id} renderItem={item => <Text>{graph${".left".repeat(depth)}.label}</Text>}/>;`;
  const transformed = compileNativeLists(ts, `depth-${depth}.tsx`, source);
  return { depth, compilerEligible: source !== transformed, source, transformed };
});
await Bun.write(new URL("compiler-depth-observation.json", import.meta.url), JSON.stringify(cases, null, 2) + "\n");
console.log(JSON.stringify(cases.map(({depth, compilerEligible}) => ({depth, compilerEligible}))));
