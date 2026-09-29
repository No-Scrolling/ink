// Prepare a direct compiler-to-QuickJS observation of the public List API.
import ts from "typescript";
import { compileNativeLists } from "../../../crates/ink-compiler/src/native-lists.js";
const depths = [4, 8, 12, 16, 20, 24, 64];
const prelude = `import {List,Screen,Text} from "ink";
import {render} from "ink/renderer";
import {createElement} from "../../../packages/ink/src/react";
const errors: string[] = [], commits: any[][] = [], output: any[] = [];
const originalPost = (globalThis as any).__inkPost;
(globalThis as any).__inkPost = (message: string) => { const value = JSON.parse(message); if(value.type === "error") errors.push(value.message); else originalPost(message); };
const originalCommit = (globalThis as any).__inkCommit;
(globalThis as any).__inkCommit = (batch: any[]) => { commits.push(batch); originalCommit(batch); };
const pause = () => new Promise(resolve => setTimeout(resolve,25));
`;
const cases = depths.map(depth => `async function case${depth}(shared:boolean,mode:string) {
 let graph:any = {label:"Leaf"}; for(let step=0;step<${depth};step++) graph=shared?{left:graph,right:graph}:{left:graph};
 const items=Array.from({length:100},(_,index)=>({id:"row-"+index}));
 const App=()=> <Screen>{mode==="compiled"
 ? <List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{graph${".left".repeat(depth)}.label}</Text>}/>
 : <List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{graph${".left".repeat(depth)}.label}</Text>;}}/>}</Screen>;
 errors.length=0;commits.length=0;
 const stop=render(createElement(App)); await pause();
 output.push({depth:${depth},distinctObjects:${depth+1},shared,mode,errors:[...errors],nativeListNodes:commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length});
 stop(); await pause();
}`).join('\n');
const footer = `(async()=>{for(const shared of [false,true])for(const mode of ["react","compiled"]){${depths.map(depth=>`await case${depth}(shared,mode);`).join('')}} originalPost(JSON.stringify({type:"audit-final-proxy",diagnostic:"actual-compiled-capture-depth",results:output}));})().catch(error=>originalPost(JSON.stringify({type:"audit-final-error",message:String(error),stack:error.stack})));`;
const source = prelude + cases + '\n' + footer;
const transformed = compileNativeLists(ts, "compiled-capture-depth.tsx", source);
const directory = new URL('.', import.meta.url);
await Bun.write(new URL('compiled-capture-depth-source.tsx', directory), source);
await Bun.write(new URL('compiled-capture-depth-runtime.tsx', directory), transformed);
console.log(JSON.stringify({ compilerPlans: (transformed.match(/__inkListRow\(/g) ?? []).length, expected: depths.length }));
const built = await Bun.build({
  entrypoints: [new URL('compiled-capture-depth-runtime.tsx', directory).pathname],
  outdir: new URL('compiled-capture-depth-bundle', directory).pathname,
  target: 'browser', format: 'iife', define: { 'process.env.NODE_ENV': '"production"' },
  plugins: [{ name: 'observation-local-resolution', setup(builder) {
    const root = '/Users/vandam/Developer/ink/packages/ink';
    builder.onResolve({ filter: /^ink(?:\/renderer|\/internal\/list)?$/ }, args => ({ path:
      root + (args.path === 'ink' ? '/src/index.ts' : args.path === 'ink/renderer' ? '/src/renderer.ts' : '/src/native-list.ts') }));
    builder.onResolve({ filter: /^react(?:\/jsx-runtime)?$/ }, args => ({ path: Bun.resolveSync(args.path, root) }));
  } }],
});
if (!built.success) throw new Error(built.logs.map(String).join('\n'));
console.log(JSON.stringify({ outputs: built.outputs.map(artifact => ({ path: artifact.path, size: artifact.size })) }));
