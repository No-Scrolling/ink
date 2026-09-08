import { useEffect, useState } from "react";
import { microphone, PitchIndicator, usePitchDetector } from "@ink/audio/microphone";
import {
  Button, ErrorState, Field, LoadingState, Navigator, Route, Screen,
  Stack, Text, Toggle, back, navigate, useAction, useSnapshot,
} from "ink";
import { TextInput } from "ink/input/numeric";
import { preferences, type Preferences } from "./settings";
import { settings } from "ink/icons";

const flatNames: Record<string, string> = {
  "C#": "D♭", "D#": "E♭", "F#": "G♭", "G#": "A♭", "A#": "B♭",
};
const noteNames = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

export default function App() {
  const saved = useSnapshot(preferences);
  const reload = useAction(preferences.get);
  const reset = useAction(preferences.reset);
  if (saved.status !== "ready") {
    return (
      <Screen title="Tuner">
        {saved.status === "loading" ? <LoadingState label="Loading settings…" /> : <>
          <ErrorState message={saved.error.message} onRetry={() => reload.run()}
            disabled={reload.status === "pending"} />
          <Button disabled={reset.status === "pending"} onPress={() => reset.run()}>Reset settings</Button>
          {reset.status === "error" && <Text>{reset.error.message}</Text>}
        </>}
      </Screen>
    );
  }
  return (
    <Navigator>
      <Route path="/"><Tuner {...saved.data} /></Route>
      <Route path="/settings"><Settings values={saved.data} /></Route>
      <Route path="/reference"><ReferencePitch initial={saved.data.referenceHz} /></Route>
    </Navigator>
  );
}

function Tuner({ referenceHz, flats, showCents, showFrequency }: Preferences) {
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

function Settings({ values }: { values: Preferences }) {
  const save = useAction((key: "flats" | "showCents" | "showFrequency", value: boolean) =>
    preferences.update(current => ({ ...current, [key]: value })));
  return (
    <Screen title="Settings">
      <Field label="Reference pitch" href="/reference">A4 · {values.referenceHz} Hz</Field>
      <Toggle label="Show flats" value={values.flats} onChange={value => save.run("flats", value)} disabled={save.status === "pending"} />
      <Toggle label="Show cents" value={values.showCents} onChange={value => save.run("showCents", value)} disabled={save.status === "pending"} />
      <Toggle label="Show frequency" value={values.showFrequency} onChange={value => save.run("showFrequency", value)} disabled={save.status === "pending"} />
      {save.status === "error" && <Text size={18}>Could not save settings. Try the switch again.</Text>}
    </Screen>
  );
}

function ReferencePitch({ initial }: { initial: number }) {
  const [reference, setReference] = useState(String(initial));
  const referenceHz = Number(reference);
  const valid = reference.trim() !== "" && Number.isFinite(referenceHz) && referenceHz >= 400 && referenceHz <= 480;
  const save = useAction(async () => {
    if (!valid) return;
    await preferences.update(current => ({ ...current, referenceHz }));
    back();
  });
  return (
    <Screen title="Reference pitch">
      <TextInput value={reference} onChange={setReference} autoFocus
        suffix="Hz" action="done" onSubmit={save.run} />
      {!valid && <Text size={18}>Enter a reference pitch from 400 to 480 Hz.</Text>}
      {save.status === "error" && <Text size={18}>Could not save settings. {save.error.message}</Text>}
    </Screen>
  );
}
