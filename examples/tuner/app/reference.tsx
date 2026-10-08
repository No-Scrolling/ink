import { useState } from "react";
import { Screen, Text, TextInput, back, useAction } from "ink";
import { preferences } from "../lib/preferences";
import { useSettings } from "../lib/settings-context";

export default function ReferencePitch() {
  const { referenceHz: initial } = useSettings();
  const [reference, setReference] = useState(String(initial));
  const referenceHz = Number(reference);
  const valid =
    reference.trim() !== "" &&
    Number.isFinite(referenceHz) &&
    referenceHz >= 400 &&
    referenceHz <= 480;
  const save = useAction(async () => {
    if (!valid) return;
    await preferences.update((current) => ({ ...current, referenceHz }));
    back();
  });
  return (
    <Screen title="Reference pitch">
      <TextInput
        inputMode="numeric"
        value={reference}
        onChange={setReference}
        autoFocus
        suffix="Hz"
        action="done"
        onSubmit={save.run}
      />
      {!valid && <Text size={18}>Enter a reference pitch from 400 to 480 Hz.</Text>}
      {save.status === "error" && (
        <Text size={18}>Could not save settings. {save.error.message}</Text>
      )}
    </Screen>
  );
}
