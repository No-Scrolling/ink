import { useEffect, useState } from "react";
import { PlayingScreen } from "ink";
import { playbackController } from "ink/internal/playback";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;
export default function App() {
  const [playing, setPlaying] = useState(false);
  const [position, setPosition] = useState(0);
  const [track, setTrack] = useState(0);
  useEffect(() => {
    const stop = onNativeMessage("benchmark", (message) => {
      if (message.command === "state") {
        setPlaying(message.playing === true);
        setPosition(Number(message.position));
        setTrack(Number(message.track));
      }
    });
    __inkPost(JSON.stringify({ type: "ready" }));
    return stop;
  }, []);
  return (
    <PlayingScreen
      title={track ? "Northern Lines" : "Harbour Lights"}
      artists={[{ name: "Málaga Ensemble" }]}
      playback={{
        state: { position, duration: 240000, playWhenReady: playing },
        [playbackController]: 7,
        toggle: () => setPlaying((previous) => !previous),
        seek: setPosition,
      }}
    />
  );
}
