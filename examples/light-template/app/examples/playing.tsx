import { useEffect, useState } from "react";
import { Button, navigate, PlayingScreen, Screen } from "ink";
import wallsocket from "../../assets/images/wallsocket.jpg";
import { repeatFilled, repeatOneFilled, shuffleFilled } from "ink/icons";

export default function Playing() {
  return <Screen title="Playing screen">
    <Button href="/examples/playing/image">Image</Button>
    <Button href="/examples/playing/no-image">No Image</Button>
  </Screen>;
}

export function PlayingExample({ image = false }: { image?: boolean }) {
  const [playing, setPlaying] = useState(false);
  const [position, setPosition] = useState(42000);
  const [repeat, setRepeat] = useState<"off" | "all" | "one">("off");
  const [shuffle, setShuffle] = useState(false);
  const duration = 213000;
  useEffect(() => {
    if (!playing) return;
    const timer = setInterval(() => setPosition(current => Math.min(current + 1000, duration)), 1000);
    return () => clearInterval(timer);
  }, [playing]);
  useEffect(() => {
    if (position < duration) return;
    if (repeat !== "off") setPosition(0);
    else setPlaying(false);
  }, [position, repeat]);
  function seekBy(milliseconds: number) {
    setPosition(current => Math.max(0, Math.min(duration, current + milliseconds)));
  }

  return <PlayingScreen image={image ? wallsocket : undefined}
    title="Cops and robbers" onTitlePress={() => navigate("/display/local-images/wallsocket")}
    artists={[{ name: "underscores", onPress: () => navigate("/actions") }]}
    playback={{ state: { playWhenReady: playing, position, duration },
      toggle: () => { if (position === duration) setPosition(0); setPlaying(current => !current); }, seek: setPosition }}
    previous={{
      seconds: image ? undefined : 10,
      onPress: () => image ? setPosition(0) : seekBy(-10000),
      onLongPress: () => seekBy(-15000),
    }}
    next={{
      seconds: image ? undefined : 30,
      onPress: () => image ? setPosition(0) : seekBy(30000),
      onLongPress: () => seekBy(15000),
    }}
    actions={[
      { icon: shuffleFilled, selected: shuffle, onPress: () => setShuffle(current => !current) },
      { icon: repeat === "one" ? repeatOneFilled : repeatFilled, selected: repeat !== "off",
        onPress: () => setRepeat(current => current === "off" ? "all" : current === "all" ? "one" : "off") },
    ]} />;
}
