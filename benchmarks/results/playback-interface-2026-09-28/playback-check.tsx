import { useRef, useState } from "react";
import { PlayingScreen, type Playback } from "ink";
export default function Check() {
 const [state,setState]=useState({position:45000,duration:120000,playWhenReady:false});
 const fail=useRef(true);
 const playback:Playback={state,
  async toggle(){if(fail.current){fail.current=false;console.log("BINDING toggle failed");throw new Error("Expected toggle failure");} console.log("BINDING toggle retried");setState(value=>({...value,playWhenReady:!value.playWhenReady}));},
  seek(position){console.log(`BINDING seek ${position}`);setState(value=>({...value,position}));},
  seekBy(offset){console.log(`BINDING skip ${offset}`);setState(value=>({...value,position:Math.max(0,Math.min(value.duration,value.position+offset))}));}
 };
 return <PlayingScreen playback={playback} title="Binding checks" artists={[]} previous={{seconds:10}} next={{seconds:30}} />;
}
