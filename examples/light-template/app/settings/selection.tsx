import { useRef } from "react";
import { ErrorState, LoadingState, Screen, SettingsChoices, back, useAction, useSnapshot } from "ink";
import { selection, type Selection as Choice } from "../../data/settings";

export default function Selection() {
  const choice = useSnapshot(selection);
  const reload = useAction(selection.get);
  const attempted = useRef<Choice>("Option 1");
  const save = useAction(async (value: Choice) => {
    attempted.current = value;
    await selection.set(value);
    back();
  });
  return (
    <Screen title="Selection">
      {choice.status === "loading" && <LoadingState label="Loading selection…" />}
      {choice.status === "ready" && <SettingsChoices value={choice.data}
        options={[{ value: "Option 1", label: "Option 1" }, { value: "Option 2", label: "Option 2" }]}
        onChange={save.run} />}
      {save.status === "pending" && <LoadingState label="" />}
      {save.status === "error" && <ErrorState message={`Could not save the selection. ${save.error.message}`}
        onRetry={() => save.run(attempted.current)} />}
      {choice.status === "error" && <ErrorState message={`Could not load the selection. ${choice.error.message}`}
        onRetry={() => reload.run()} />}
    </Screen>
  );
}
