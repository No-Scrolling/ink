import { useEffect, useState } from "react";
import { usePlayer } from "@ink/audio";
import { Screen, Toggle, Text, LoadingState, useAction } from "ink";

export default function Effects() {
  const player = usePlayer({ session: "main", mode: "detached" });
  const [silenceSaved, setSilenceSaved] = useState(player.state.silenceSaved);
  useEffect(() => {
    if (!player.state.ready) return;
    let active = true;
    const refresh = async () => {
      try {
        const state = await player.getState();
        if (active) setSilenceSaved(state.silenceSaved);
      } catch (error) {
        if (active) console.error("Could not read playback effects", error);
      }
    };
    void refresh();
    const timer = setInterval(() => {
      void refresh();
    }, 1000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [player.state.ready, player.getState]);
  const silence = useAction((enabled: boolean) => player.setSkipSilence(enabled));
  const voice = useAction((enabled: boolean) => player.setVoiceBoost(enabled));
  const error =
    player.state.error ??
    (silence.status === "error" ? silence.error : null) ??
    (voice.status === "error" ? voice.error : null);

  return (
    <Screen title="Effects">
      {!player.state.ready ? (
        <LoadingState />
      ) : (
        <>
          <Toggle
            label="Smart speed"
            value={player.state.skipSilence}
            onChange={silence.run}
            subtitle={`Total time saved: ${Math.floor(silenceSaved / 1000)} seconds`}
          />
          <Toggle label="Voice boost" value={player.state.voiceBoost} onChange={voice.run} />
        </>
      )}
      {error && <Text>{error.message}</Text>}
    </Screen>
  );
}
