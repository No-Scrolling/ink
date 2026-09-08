import { useAction } from "ink";
import { useCallback, useRef } from "react";
import { preferences, type WeatherPreferences } from "../lib/preferences";

export function useSettingsUpdate({ onSuccess }: { onSuccess?: () => void } = {}) {
  const attempted = useRef<Partial<WeatherPreferences>>({});
  const operation = useCallback(
    async (changes: Partial<WeatherPreferences>) => {
      attempted.current = changes;
      await preferences.update(current => ({ ...current, ...changes }));
      onSuccess?.();
    },
    [onSuccess],
  );
  const action = useAction(operation);
  return { ...action, retry: () => action.run(attempted.current) };
}
