import { useState } from "react";
import { Button, Screen, Text } from "ink";
import { usePodcastPlayer } from "../lib/player";
const wait = (ms:number) => new Promise<void>(resolve=>setTimeout(()=>resolve(),ms));
export default function Check() {
 const player=usePodcastPlayer(); const [result,setResult]=useState("Ready");
 async function run() {
  try {
   setResult("Running");
   const src="http://10.0.2.2:8787/audio.mp3";
   await player.setQueue([{id:"first",src,title:"First"},{id:"second",src,title:"Second"}],{startPosition:30000});
   await player.play();
   for(let i=0;i<50 && !(await player.getState()).playing;i++) await wait(100);
   await player.pause();
   const before=await player.getState();
   await player.replaceSource("first",src+"?replacement");
   await wait(300);
   const after=await player.getState();
   if(after.playWhenReady || Math.abs(after.position-before.position)>100) throw new Error("Replacement lost pause or position");
   await player.replaceSource("first",src,{offset:10000,prepare:false});
   const shifted=await player.getState();
   if(shifted.playWhenReady || Math.abs(shifted.position-before.position-10000)>100) throw new Error("Replacement offset failed");
   await player.next();
   if((await player.getState()).current?.id!=="second") throw new Error("Replacement lost the queue");
   let rejected=false;
   try { await player.replaceSource("first",src); } catch { rejected=true; }
   if(!rejected || (await player.getState()).current?.id!=="second") throw new Error("Stale ID replaced wrong track");
   await player.setQueue([{id:"second",src,title:"Second"}]);
   setResult("PASS: position, pause, offset, queue, stale ID"); console.log("SOURCE PASS");
  } catch(error) {setResult(String(error));console.error("SOURCE FAIL",error);}
 }
 return <Screen title="Source checks"><Text>{result}</Text><Button disabled={!player.state.ready} onPress={run}>Run source checks</Button></Screen>;
}
