import { useEffect, useState } from "react";
import { microphone, PitchIndicator, usePitchDetector } from "@ink/audio/microphone";
import { Screen, Stack, Text, navigate } from "ink";
import { settings } from "ink/icons";
import { useSettings } from "../lib/settings-context";

const flatNames: Record<string, string> = {
  "C#": "D♭", "D#": "E♭", "F#": "G♭", "G#": "A♭", "A#": "B♭",
};
const noteNames = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

export default function Tuner() {
  const { referenceHz, flats, showCents, showFrequency } = useSettings();
  const pitch = usePitchDetector({ referenceHz });
  const [lastFrequency, setLastFrequency] = useState<number | null>(null);
  const frequency = pitch.state.status === "active" && pitch.state.frequency > 0
    ? pitch.state.frequency : lastFrequency;
  useEffect(() => {
    if (pitch.state.status === "active" && pitch.state.frequency > 0) {
      setLastFrequency(pitch.state.frequency);
    }
  }, [pitch.state.status, pitch.state.frequency]);
  const [permissionMessage, setPermissionMessage] = useState<string | null>(null);
  const [initialError, setInitialError] = useState<string | null>(null);
  useEffect(() => {
    if (!pitch.ready) return;
    let cancelled = false;
    setInitialError(null);
    setPermissionMessage(null);
    void (async () => {
      try {
        let permission = await microphone.getPermission();
        if (cancelled) return;
        if (permission === "denied") permission = await microphone.requestPermission();
        if (cancelled) return;
        if (permission === "granted") await pitch.start();
        else setPermissionMessage(permission === "blocked"
          ? "Allow microphone access in your phone’s app settings, then reopen Tuner."
          : "Microphone access is needed to hear a note. Reopen Tuner to try again.");
      } catch (error) {
        if (!cancelled) setInitialError(error instanceof Error ? error.message : String(error));
      }
    })();
    return () => { cancelled = true; };
  }, [pitch.ready, pitch.start]);

  const midi = frequency === null ? 69 : 69 + 12 * Math.log2(frequency / referenceHz);
  const nearest = Math.round(midi);
  const deviation = (midi - nearest) * 100;
  const cents = Math.round(deviation);
  const name = noteNames[((nearest % 12) + 12) % 12];
  const note = flats ? flatNames[name] ?? name : name.replace("#", "♯");
  const octave = Math.floor(nearest / 12) - 1;
  const error = initialError ?? pitch.state.error?.message;
  const readings = [];
  if (showCents) readings.push(frequency !== null ? `${cents > 0 ? "+" : ""}${cents} cents` : "— cents");
  if (showFrequency) readings.push(frequency !== null ? `${frequency.toFixed(1)} Hz` : "— Hz");

  return (
    <Screen title="Tuner" centered rightAction={{ icon: settings, onPress: () => navigate("/settings") }}>
      {error ? <Text size={20} align="center">{error}</Text>
        : permissionMessage ? <Text size={20} align="center">{permissionMessage}</Text>
        : <Stack gap={14} align="center">
          <Text size={100} align="center">{frequency !== null ? `${note}${octave}` : "—"}</Text>
          <PitchIndicator cents={frequency !== null ? deviation : null} />
          {readings.length > 0 && <Text size={20} align="center" tabularNumbers>{readings.join(" · ")}</Text>}
        </Stack>}
    </Screen>
  );
}
