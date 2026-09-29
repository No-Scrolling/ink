import { nativeListRow as __inkListRow, nativeListKey as __inkListKey, listText as __inkListText, listRowFields as __inkListFields } from "ink/internal/list";
import {List,Screen,Text} from "ink";
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
 ? mode==="compiled"?<List items={items} keyExtractor={__inkListKey(item=>item.id, "function-property.tsx:1513")} renderItem={__inkListRow(item=><Text onPress={config}>Callback</Text>, (item) => [config,__inkListText(["Callback"])], "{\"id\":0,\"type\":\"Stack\",\"props\":{\"gap\":0},\"children\":[{\"id\":1,\"type\":\"Text\",\"props\":{\"onPress\":{\"$value\":0,\"path\":[\"values\",0]},\"text\":{\"$value\":0,\"path\":[\"values\",1]}},\"children\":[]}]}", {identity:"function-property.tsx:1540",values:() => [config]})}/>:<List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text onPress={config}>Callback</Text>;}}/>
 : mode==="compiled"?<List items={items} keyExtractor={__inkListKey(item=>item.id, "function-property.tsx:1762")} renderItem={__inkListRow(item=><Text>{kind==="prototype-getter"||kind==="prototype-data"?config.prototype.label:config.label}</Text>, (item) => [__inkListText([kind==="prototype-getter"||kind==="prototype-data"?config.prototype.label:config.label])], "{\"id\":0,\"type\":\"Stack\",\"props\":{\"gap\":0},\"children\":[{\"id\":1,\"type\":\"Text\",\"props\":{\"text\":{\"$value\":0,\"path\":[\"values\",0]}},\"children\":[]}]}", {identity:"function-property.tsx:1789",values:() => [kind,config]})}/>:<List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{kind==="prototype-getter"||kind==="prototype-data"?config.prototype.label:config.label}</Text>;}}/>}</Screen>;}
 commits.length=0;errors.length=0;const stop=render(createElement(App));await pause();
 const initialReads=reads,nativeListNodes=commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length;
 commits.length=0;if(kind==="data")config.label="After";else if(kind==="prototype-data")config.prototype.label="After";refresh!();await pause();
 output.push({kind,mode,initialReads,updateReads:reads-initialReads,nativeListNodes,errors:[...errors],updateOperations:commits.flat()});
 stop();await pause();
}
(async()=>{for(const kind of ["getter","data","prototype-getter","prototype-data","cycle","plain-callback","arrow-callback","bound-callback"])for(const mode of ["react","compiled"])await inspect(kind,mode);originalPost(JSON.stringify({type:"audit-final-proxy",diagnostic:"actual-compiled-function-property",results:output}));})().catch(error=>originalPost(JSON.stringify({type:"audit-final-error",message:String(error),stack:error.stack})));