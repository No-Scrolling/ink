import { useState } from "react";
import { ErrorState, LoadingState, Screen, SettingsChoices, back, useAction, useSnapshot } from "ink";
import { selection, type Selection as Choice } from "../../data/settings";

export default function Selection() {
  const choice = useSnapshot(selection);
  const reload = useAction(selection.get);
  const [attempted, setAttempted] = useState<Choice>("Option 1");
  const save = useAction(async (value: Choice) => {
    await selection.set(value);
    back();
  });
  function choose(value: Choice) { setAttempted(value); save.run(value); }
  return (
    <Screen title="Selection">
      {choice.status === "loading" && <LoadingState label="Loading selection…" />}
      {choice.status === "ready" && <SettingsChoices value={choice.data}
        options={[{ value: "Option 1", label: "Option 1" }, { value: "Option 2", label: "Option 2" }]}
        disabled={save.status === "pending"} onChange={choose} />}
      {save.status === "pending" && <LoadingState label="Saving selection…" />}
      {save.status === "error" && <ErrorState message={`Could not save the selection. ${save.error.message}`}
        onRetry={() => save.run(attempted)} />}
      {choice.status === "error" && <ErrorState message={`Could not load the selection. ${choice.error.message}`}
        onRetry={() => reload.run()} disabled={reload.status === "pending"} />}
    </Screen>
  );
}
