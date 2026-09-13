import { ErrorState, Field, LoadingState, Screen, Toggle, useAction, useSnapshot } from "ink";
import { appearance } from "../../data/appearance";
import { selection } from "../../data/settings";

export default function Settings() {
  const saved = useSnapshot(appearance);
  const choice = useSnapshot(selection);
  const reloadAppearance = useAction(appearance.get);
  const reloadSelection = useAction(selection.get);
  const saveAppearance = useAction((light: boolean) => appearance.set(light ? "light" : "dark"));
  return (
    <Screen title="Settings">
      {saved.status === "ready" && <Toggle label="Invert Colours" value={saved.data === "light"}
        onChange={saveAppearance.run} />}
      {saved.status === "loading" && <LoadingState label="Loading appearance…" />}
      {saved.status === "error" && <ErrorState message={saved.error.message}
        onRetry={() => reloadAppearance.run()} />}
      {saveAppearance.status === "error" && <ErrorState message={saveAppearance.error.message}
        onRetry={() => { if (saved.status === "ready") saveAppearance.run(saved.data !== "light"); }} />}
      <Field label="Selection" href="/settings/selection">
        {choice.status === "ready" ? choice.data : choice.status === "error" ? "Unavailable" : "Loading…"}
      </Field>
      {choice.status === "error" && <ErrorState message={`Could not load the selection. ${choice.error.message}`}
        onRetry={() => reloadSelection.run()} />}
    </Screen>
  );
}
