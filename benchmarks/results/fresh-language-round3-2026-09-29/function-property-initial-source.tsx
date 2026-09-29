import {List,Screen,Text} from "ink";
import {render} from "ink/renderer";
import {createElement,useState} from "../../../packages/ink/src/react";
const originalPost = (globalThis as any).__inkPost, originalCommit = (globalThis as any).__inkCommit;
const commits:any[][]=[],errors:string[]=[],output:any[]=[];
(globalThis as any).__inkCommit=(batch:any[])=>{commits.push(batch);originalCommit(batch);};
(globalThis as any).__inkPost=(message:string)=>{const value=JSON.parse(message);if(value.type==="error")errors.push(value.message);else originalPost(message);};
const pause=()=>new Promise(resolve=>setTimeout(resolve,25));
async function inspect(kind:string,mode:string){
 let reads=0;const config:any=()=>{};
 if(kind==="getter")Object.defineProperty(config,"label",{get(){reads++;return "Getter label";}});else config.label="Before";
 const items=Array.from({length:1000},(_,index)=>({id:"row-"+index}));
 let refresh:()=>void;
 function App(){const[,setTick]=useState(0);refresh=()=>setTick(value=>value+1);return <Screen>{mode==="compiled"
 ? <List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{config.label}</Text>}/>
 : <List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{config.label}</Text>;}}/>}</Screen>;}
 commits.length=0;errors.length=0;const stop=render(createElement(App));await pause();
 const initialReads=reads,nativeListNodes=commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length;
 commits.length=0;if(kind==="data")config.label="After";refresh!();await pause();
 output.push({kind,mode,initialReads,updateReads:reads-initialReads,nativeListNodes,errors:[...errors],updateOperations:commits.flat()});
 stop();await pause();
}
(async()=>{for(const kind of ["getter","data"])for(const mode of ["react","compiled"])await inspect(kind,mode);originalPost(JSON.stringify({type:"audit-final-proxy",diagnostic:"actual-compiled-function-property",results:output}));})().catch(error=>originalPost(JSON.stringify({type:"audit-final-error",message:String(error),stack:error.stack})));