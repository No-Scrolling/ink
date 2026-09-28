import { useRef, useState } from "react";
import { Button, Screen, Text, navigate } from "ink";
import { usePodcastPlayer } from "../lib/player";
import { downloadEpisode, downloads } from "../lib/downloads";
import { episodeProgress } from "../lib/progress";
const wait = (ms: number) => new Promise<void>(resolve => setTimeout(() => resolve(), ms));
function assert(condition: boolean, message: string) { if (!condition) throw new Error(message); }
export default function Check() {
  const player = usePodcastPlayer();
  const latest = useRef(player); latest.current = player;
  const [result, setResult] = useState("Ready");
  async function run() {
    try {
      setResult("Running");
      await player.setQueue([{id:"migration",src:"http://10.0.2.2:8787/audio.mp3",title:"Migration check"}]);
      await player.play();
      for (let i=0;i<50 && !(await player.getState()).playing;i++) await wait(100);
      assert((await player.getState()).playing,"Playback did not start");
      await wait(600);
      const before = await player.getState();
      const stale = latest.current.state.position;
      await wait(1600);
      const fresh = await player.getState();
      assert(fresh.position > before.position+1000,"Fresh read did not advance");
      assert(latest.current.state.position === stale,"Periodic React progress returned");
      await player.seekBy(15000);
      for (let i=0;i<50 && !(await player.getState()).playing;i++) await wait(100);
      const skipped = await player.getState();
      assert(skipped.position >= fresh.position+14500,`Relative seek used stale position: ${JSON.stringify({fresh,skipped})}`);
      await player.pause(); await wait(200);
      const paused = await player.getState(); await wait(700);
      assert(Math.abs((await player.getState()).position-paused.position)<100,"Pause did not freeze");
      const history=episodeProgress.getSnapshot();
      assert(history.status==="ready" && Math.abs(history.data.migration.position-paused.position)<500,"Pause checkpoint incorrect");
      await player.seekBy(-1000000);
      assert((await player.getState()).position===0,"Negative seek did not clamp");
      await player.setSpeed(2); await player.play();
      for (let i=0;i<50 && !(await player.getState()).playing;i++) await wait(100);
      await wait(500);
      const fast=await player.getState(); await wait(1000);
      const fastAfter=await player.getState();
      assert(fastAfter.position>fast.position+1500,`Playback speed incorrect: ${JSON.stringify({fast,fastAfter})}`);
      await player.pause(); await player.setSpeed(1);
      await downloadEpisode("migration", "http://10.0.2.2:8787/audio.mp3", "Migration check");
      let downloaded = false;
      for (let i=0;i<100;i++) {
        await downloads.refresh();
        const state=downloads.getSnapshot();
        if (state.status==="ready" && state.data.some(item=>item.id==="migration" && item.status==="finished")) { downloaded=true; break; }
        await wait(100);
      }
      assert(downloaded,"Download did not finish");
      const old=await player.getState();
      await player.setQueue([{...old.current!,src:"http://10.0.2.2:8787/audio.mp3"}],{startPosition:old.position});
      const sourceBefore=await player.getState();
      await player.toggle(); await wait(300);
      const local=await player.getState();
      assert(local.current?.src!==sourceBefore.current?.src,"Downloaded source not selected");
      assert(Math.abs(local.position-sourceBefore.position)<1500,"Source switch lost position");
      await player.seek(30000); await player.play();
      setResult("Checking 30-second checkpoint");
      await wait(31500);
      const saved=await episodeProgress.get();
      assert(saved.migration.position>31000,"Periodic checkpoint did not read fresh position");
      await player.pause();
      const duration=(await player.getState()).duration;
      await player.seek(duration-500); await player.play(); await wait(1500);
      const ended=await player.getState();
      assert(ended.ended,"End notification missing");
      assert((await episodeProgress.get()).migration.finished,"Finished progress not saved");
      setResult("PASS: fresh reads, no ticks, skip, pause, save, speed, download switch, checkpoint, ended");
      console.log("MIGRATION PASS podcasts");
    } catch (error) { setResult(String(error)); console.error("MIGRATION FAIL",error); }
  }
  return <Screen title="Migration checks"><Text>{result}</Text><Button disabled={!player.state.ready} onPress={run}>Run checks</Button><Button onPress={()=>navigate("/playing")}>Playing screen</Button><Button onPress={()=>navigate("/effects")}>Effects</Button></Screen>;
}
