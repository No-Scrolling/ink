import { useAction } from "ink";
import { useRef } from "react";
import { preferences, type WeatherPreferences } from "../lib/preferences";

export function useSettingsUpdate({ onSuccess }: { onSuccess?: () => void } = {}) {
  const attempted = useRef<Partial<WeatherPreferences>>({});
  const operation = async (changes: Partial<WeatherPreferences>) => {
    attempted.current = changes;
    await preferences.update((current) => ({ ...current, ...changes }));
    onSuccess?.();
  };
  const action = useAction(operation);
  return { ...action, retry: () => action.run(attempted.current) };
}
