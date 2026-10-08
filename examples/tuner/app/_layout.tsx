import { Button, ErrorState, LoadingState, Screen, Slot, Text, useAction, useSnapshot } from "ink";
import { preferences } from "../lib/preferences";
import { SettingsContext } from "../lib/settings-context";

export default function Layout() {
  const saved = useSnapshot(preferences);
  const reload = useAction(preferences.get);
  const reset = useAction(preferences.reset);
  if (saved.status !== "ready") {
    return (
      <Screen title="Tuner">
        {saved.status === "loading" ? (
          <LoadingState label="Loading settings…" />
        ) : (
          <>
            <ErrorState message={saved.error.message} onRetry={() => reload.run()} />
            <Button onPress={() => reset.run()}>Reset settings</Button>
            {reset.status === "error" && <Text>{reset.error.message}</Text>}
          </>
        )}
      </Screen>
    );
  }
  return (
    <SettingsContext value={saved.data}>
      <Slot />
    </SettingsContext>
  );
}
