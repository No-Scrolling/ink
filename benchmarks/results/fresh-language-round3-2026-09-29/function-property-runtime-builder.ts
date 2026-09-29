import ts from "typescript";
import { compileNativeLists } from "../../../crates/ink-compiler/src/native-lists.js";
const source = `import {List,Screen,Text} from "ink";
import {render} from "ink/renderer";
import {createElement,useState} from "../../../packages/ink/src/react";
const originalPost = (globalThis as any).__inkPost, originalCommit = (globalThis as any).__inkCommit;
const commits:any[][]=[],errors:string[]=[],output:any[]=[];
(globalThis as any).__inkCommit=(batch:any[])=>{commits.push(batch);originalCommit(batch);};
(globalThis as any).__inkPost=(message:string)=>{const value=JSON.parse(message);if(value.type==="error")errors.push(value.message);else originalPost(message);};
const pause=()=>new Promise(resolve=>setTimeout(resolve,25));
async function inspect(kind:string,mode:string){
 let reads=0;const config:any=kind==="bound-callback"?(function handler(){}).bind(null):kind==="arrow-callback"?()=>{}:function handler(){};
 if(kind==="getter")Object.defineProperty(config,"label",{get(){reads++;return "Getter label";}});
 else if(kind==="prototype-getter")Object.defineProperty(config.prototype,"label",{get(){reads++;return "Prototype label";}});
 else if(kind==="prototype-data")config.prototype.label="Before";
 else if(kind==="cycle"){config.metadata={};config.metadata.self=config.metadata;config.label="Cycle";}
 else config.label="Before";
 const items=Array.from({length:1000},(_,index)=>({id:"row-"+index}));
 let refresh:()=>void;
 function App(){const[,setTick]=useState(0);refresh=()=>setTick(value=>value+1);return <Screen>{kind.endsWith("callback")
 ? mode==="compiled"?<List items={items} keyExtractor={item=>item.id} renderItem={item=><Text onPress={config}>Callback</Text>}/>:<List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text onPress={config}>Callback</Text>;}}/>
 : mode==="compiled"?<List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{kind==="prototype-getter"||kind==="prototype-data"?config.prototype.label:config.label}</Text>}/>:<List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{kind==="prototype-getter"||kind==="prototype-data"?config.prototype.label:config.label}</Text>;}}/>}</Screen>;}
 commits.length=0;errors.length=0;const stop=render(createElement(App));await pause();
 const initialReads=reads,nativeListNodes=commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length;
 commits.length=0;if(kind==="data")config.label="After";else if(kind==="prototype-data")config.prototype.label="After";refresh!();await pause();
 output.push({kind,mode,initialReads,updateReads:reads-initialReads,nativeListNodes,errors:[...errors],updateOperations:commits.flat()});
 stop();await pause();
}
(async()=>{for(const kind of ["getter","data","prototype-getter","prototype-data","cycle","plain-callback","arrow-callback","bound-callback"])for(const mode of ["react","compiled"])await inspect(kind,mode);originalPost(JSON.stringify({type:"audit-final-proxy",diagnostic:"actual-compiled-function-property",results:output}));})().catch(error=>originalPost(JSON.stringify({type:"audit-final-error",message:String(error),stack:error.stack})));`;
const transformed = compileNativeLists(ts, "function-property.tsx", source);
const directory = new URL('.', import.meta.url);
await Bun.write(new URL('function-property-source.tsx', directory), source);
await Bun.write(new URL('function-property-runtime.tsx', directory), transformed);
const built=await Bun.build({entrypoints:[new URL('function-property-runtime.tsx',directory).pathname],outdir:new URL('function-property-bundle',directory).pathname,target:'browser',format:'iife',define:{'process.env.NODE_ENV':'"production"'},plugins:[{name:'observation-local-resolution',setup(builder){const root='/Users/vandam/Developer/ink/packages/ink';builder.onResolve({filter:/^ink(?:\/renderer|\/internal\/list)?$/},args=>({path:root+(args.path==='ink'?'/src/index.ts':args.path==='ink/renderer'?'/src/renderer.ts':'/src/native-list.ts')}));builder.onResolve({filter:/^react(?:\/jsx-runtime)?$/},args=>({path:Bun.resolveSync(args.path,root)}));}}]});
if(!built.success)throw new Error(built.logs.map(String).join('\n'));
console.log(JSON.stringify({compilerPlans:(transformed.match(/__inkListRow\(/g)??[]).length,outputs:built.outputs.map(artifact=>({path:artifact.path,size:artifact.size}))}));
