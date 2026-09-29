import {List,Screen,Text} from "ink";
import {render} from "ink/renderer";
import {createElement} from "../../../packages/ink/src/react";
const errors: string[] = [], commits: any[][] = [], output: any[] = [];
const originalPost = (globalThis as any).__inkPost;
(globalThis as any).__inkPost = (message: string) => { const value = JSON.parse(message); if(value.type === "error") errors.push(value.message); else originalPost(message); };
const originalCommit = (globalThis as any).__inkCommit;
(globalThis as any).__inkCommit = (batch: any[]) => { commits.push(batch); originalCommit(batch); };
const pause = () => new Promise(resolve => setTimeout(resolve,25));
async function case4(shared:boolean,mode:string) {
 let graph:any = {label:"Leaf"}; for(let step=0;step<4;step++) graph=shared?{left:graph,right:graph}:{left:graph};
 const items=Array.from({length:100},(_,index)=>({id:"row-"+index}));
 const App=()=> <Screen>{mode==="compiled"
 ? <List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{graph.left.left.left.left.label}</Text>}/>
 : <List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{graph.left.left.left.left.label}</Text>;}}/>}</Screen>;
 errors.length=0;commits.length=0;
 const stop=render(createElement(App)); await pause();
 output.push({depth:4,distinctObjects:5,shared,mode,errors:[...errors],nativeListNodes:commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length});
 stop(); await pause();
}
async function case8(shared:boolean,mode:string) {
 let graph:any = {label:"Leaf"}; for(let step=0;step<8;step++) graph=shared?{left:graph,right:graph}:{left:graph};
 const items=Array.from({length:100},(_,index)=>({id:"row-"+index}));
 const App=()=> <Screen>{mode==="compiled"
 ? <List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{graph.left.left.left.left.left.left.left.left.label}</Text>}/>
 : <List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{graph.left.left.left.left.left.left.left.left.label}</Text>;}}/>}</Screen>;
 errors.length=0;commits.length=0;
 const stop=render(createElement(App)); await pause();
 output.push({depth:8,distinctObjects:9,shared,mode,errors:[...errors],nativeListNodes:commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length});
 stop(); await pause();
}
async function case12(shared:boolean,mode:string) {
 let graph:any = {label:"Leaf"}; for(let step=0;step<12;step++) graph=shared?{left:graph,right:graph}:{left:graph};
 const items=Array.from({length:100},(_,index)=>({id:"row-"+index}));
 const App=()=> <Screen>{mode==="compiled"
 ? <List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>}/>
 : <List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>;}}/>}</Screen>;
 errors.length=0;commits.length=0;
 const stop=render(createElement(App)); await pause();
 output.push({depth:12,distinctObjects:13,shared,mode,errors:[...errors],nativeListNodes:commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length});
 stop(); await pause();
}
async function case16(shared:boolean,mode:string) {
 let graph:any = {label:"Leaf"}; for(let step=0;step<16;step++) graph=shared?{left:graph,right:graph}:{left:graph};
 const items=Array.from({length:100},(_,index)=>({id:"row-"+index}));
 const App=()=> <Screen>{mode==="compiled"
 ? <List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>}/>
 : <List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>;}}/>}</Screen>;
 errors.length=0;commits.length=0;
 const stop=render(createElement(App)); await pause();
 output.push({depth:16,distinctObjects:17,shared,mode,errors:[...errors],nativeListNodes:commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length});
 stop(); await pause();
}
async function case20(shared:boolean,mode:string) {
 let graph:any = {label:"Leaf"}; for(let step=0;step<20;step++) graph=shared?{left:graph,right:graph}:{left:graph};
 const items=Array.from({length:100},(_,index)=>({id:"row-"+index}));
 const App=()=> <Screen>{mode==="compiled"
 ? <List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>}/>
 : <List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>;}}/>}</Screen>;
 errors.length=0;commits.length=0;
 const stop=render(createElement(App)); await pause();
 output.push({depth:20,distinctObjects:21,shared,mode,errors:[...errors],nativeListNodes:commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length});
 stop(); await pause();
}
async function case24(shared:boolean,mode:string) {
 let graph:any = {label:"Leaf"}; for(let step=0;step<24;step++) graph=shared?{left:graph,right:graph}:{left:graph};
 const items=Array.from({length:100},(_,index)=>({id:"row-"+index}));
 const App=()=> <Screen>{mode==="compiled"
 ? <List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>}/>
 : <List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>;}}/>}</Screen>;
 errors.length=0;commits.length=0;
 const stop=render(createElement(App)); await pause();
 output.push({depth:24,distinctObjects:25,shared,mode,errors:[...errors],nativeListNodes:commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length});
 stop(); await pause();
}
async function case64(shared:boolean,mode:string) {
 let graph:any = {label:"Leaf"}; for(let step=0;step<64;step++) graph=shared?{left:graph,right:graph}:{left:graph};
 const items=Array.from({length:100},(_,index)=>({id:"row-"+index}));
 const App=()=> <Screen>{mode==="compiled"
 ? <List items={items} keyExtractor={item=>item.id} renderItem={item=><Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>}/>
 : <List items={items} keyExtractor={item=>item.id} renderItem={item=>{return <Text>{graph.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.left.label}</Text>;}}/>}</Screen>;
 errors.length=0;commits.length=0;
 const stop=render(createElement(App)); await pause();
 output.push({depth:64,distinctObjects:65,shared,mode,errors:[...errors],nativeListNodes:commits.flat().filter(x=>x.op==="create"&&x.type==="NativeList").length});
 stop(); await pause();
}
(async()=>{for(const shared of [false,true])for(const mode of ["react","compiled"]){await case4(shared,mode);await case8(shared,mode);await case12(shared,mode);await case16(shared,mode);await case20(shared,mode);await case24(shared,mode);await case64(shared,mode);} originalPost(JSON.stringify({type:"audit-final-proxy",diagnostic:"actual-compiled-capture-depth",results:output}));})().catch(error=>originalPost(JSON.stringify({type:"audit-final-error",message:String(error),stack:error.stack})));