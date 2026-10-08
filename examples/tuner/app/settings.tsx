import { Field, Screen, Text, Toggle, useAction } from "ink";
import { preferences } from "../lib/preferences";
import { useSettings } from "../lib/settings-context";

export default function Settings() {
  const values = useSettings();
  const save = useAction((key: "flats" | "showCents" | "showFrequency", value: boolean) =>
    preferences.update((current) => ({ ...current, [key]: value })),
  );
  return (
    <Screen title="Settings">
      <Field label="Reference pitch" href="/reference">
        A4 · {values.referenceHz} Hz
      </Field>
      <Toggle
        label="Show flats"
        value={values.flats}
        onChange={(value) => save.run("flats", value)}
      />
      <Toggle
        label="Show cents"
        value={values.showCents}
        onChange={(value) => save.run("showCents", value)}
      />
      <Toggle
        label="Show frequency"
        value={values.showFrequency}
        onChange={(value) => save.run("showFrequency", value)}
      />
      {save.status === "error" && (
        <Text size={18}>Could not save settings. Try the switch again.</Text>
      )}
    </Screen>
  );
}
