import { ErrorState, LoadingState, Screen, useSnapshot } from "ink";
import { preferences, type WeatherPreferences } from "../lib/preferences";

export function PreferencesGate({
  title,
  children,
}: {
  title: string;
  children: (value: WeatherPreferences) => React.ReactNode;
}) {
  const saved = useSnapshot(preferences);
  if (saved.status === "loading") return <Screen title={title}><LoadingState label="Loading settings…" /></Screen>;
  if (saved.status === "error") {
    return <Screen title={title}><ErrorState message={saved.error.message} onRetry={() => { void preferences.get(); }} /></Screen>;
  }
  return children(saved.data);
}
