import { useState } from "react";
import { Button, Screen, Text, navigate } from "ink";
import { usePodcastPlayer } from "../lib/player";
import { downloadEpisode, downloads } from "../lib/downloads";
import { currentEpisode } from "../lib/current-episode";
import { playbackSettings } from "../lib/playback-settings";
import { episodeProgress } from "../lib/progress";
const wait = (ms:number) => new Promise<void>(resolve=>setTimeout(()=>resolve(),ms));
export default function Check() {
 const player=usePodcastPlayer(); const [result,setResult]=useState("Ready");
 async function autoplay() {
  try {
   setResult("Checking autoplay");
   await downloadEpisode("migration-next","http://10.0.2.2:8787/audio.mp3","Next episode");
   for(let i=0;i<100;i++) {
    await downloads.refresh(); const saved=downloads.getSnapshot();
    if(saved.status==="ready" && saved.data.some(item=>item.id==="migration-next" && item.status==="finished")) break;
    await wait(100);
   }
   await episodeProgress.set({});
   await currentEpisode.set({id:"migration",date:"",source:{type:"downloads",ids:["migration","migration-next"]}});
   await playbackSettings.update(value=>({...value,playNext:true}));
   await player.setQueue([{id:"migration",src:"http://10.0.2.2:8787/audio.mp3",title:"First episode"}]);
   await player.play();
   for(let i=0;i<50 && !(await player.getState()).playing;i++) await wait(100);
   await player.seek((await player.getState()).duration-500);
   for(let i=0;i<100 && (await player.getState()).current?.id!=="migration-next";i++) await wait(100);
   const next=await player.getState();
   if(next.current?.id!=="migration-next") throw new Error("Autoplay did not select next episode");
   if(!(await episodeProgress.get()).migration?.finished) throw new Error("Autoplay did not save completed episode");
   await playbackSettings.update(value=>({...value,playNext:false}));
   setResult("PASS: autoplay and finished progress");console.log("MIGRATION PASS autoplay");
  } catch(error) { setResult(String(error));console.error("MIGRATION FAIL autoplay",error); }
 }
 return <Screen title="Lifecycle checks"><Text>{result}</Text><Button disabled={!player.state.ready} onPress={autoplay}>Check autoplay</Button><Button onPress={()=>navigate("/playing")}>Playing screen</Button><Button onPress={()=>navigate({path:"/delete-download",params:{id:"migration-next"}})}>Delete playing download</Button><Button onPress={async()=>{
 await player.setQueue([{id:"silence",src:"http://10.0.2.2:8787/silence.wav",title:"Silence check"}]);
 await player.setSkipSilence(true);await player.play();navigate("/effects");
 }}>Check effects</Button></Screen>;
}
