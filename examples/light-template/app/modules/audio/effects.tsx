import { usePlayer } from "@ink/audio";
import { Screen, Toggle, Text, LoadingState, useAction } from "ink";

export default function Effects() {
  const player = usePlayer({ session: "main", mode: "detached" });
  const silence = useAction((enabled: boolean) => player.setSkipSilence(enabled));
  const voice = useAction((enabled: boolean) => player.setVoiceBoost(enabled));
  const error = player.state.error
    ?? (silence.status === "error" ? silence.error : null)
    ?? (voice.status === "error" ? voice.error : null);

  return <Screen title="Effects">
    {!player.state.ready ? <LoadingState /> : <>
      <Toggle label="Smart speed" value={player.state.skipSilence} onChange={silence.run}
        subtitle={`Total time saved: ${Math.floor(player.state.silenceSaved / 1000)} seconds`} />
      <Toggle label="Voice boost" value={player.state.voiceBoost} onChange={voice.run} />
    </>}
    {error && <Text>{error.message}</Text>}
  </Screen>;
}
