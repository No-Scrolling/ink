---
title: "Request a permission"
description: "Ask when a feature opens and handle the answer."
---

Request permission when the person opens a feature that needs it. A tuner can ask on its first screen; an app with an optional camera should ask when the camera opens. Avoid a separate button whose only purpose is to start the first permission request.

Install `@ink/audio` to use this complete microphone screen:

```tsx
import { useEffect, useState } from "react";
import { microphone, PitchIndicator, usePitchDetector } from "@ink/audio/microphone";
import { ErrorState, Screen, Stack, Text } from "ink";

export default function Tuner() {
  const pitch = usePitchDetector({ referenceHz: 440 });
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    if (!pitch.ready) return;
    let cancelled = false;
    setError(null);
    void (async () => {
      try {
        let permission = await microphone.getPermission();
        if (cancelled) return;
        if (permission === "denied") permission = await microphone.requestPermission();
        if (cancelled) return;
        if (permission !== "granted") {
          setError(permission === "blocked"
            ? "Allow microphone access in your phone’s app settings, then try again."
            : "Microphone access is needed to hear a note.");
          return;
        }
        await pitch.start();
      } catch (cause) {
        if (!cancelled) setError(cause instanceof Error ? cause.message : String(cause));
      }
    })();
    return () => { cancelled = true; };
  }, [pitch.ready, pitch.start, attempt]);
  const failure = error ?? pitch.state.error?.message;
  if (failure) return <Screen title="Tuner">
    <ErrorState message={failure} onRetry={() => setAttempt(value => value + 1)} />
  </Screen>;
  return <Screen title="Tuner" centered>
    <Stack align="center" gap={14}>
      <Text size={100}>{pitch.state.status === "active" ? pitch.state.note : "—"}</Text>
      <PitchIndicator cents={pitch.state.status === "active" ? pitch.state.cents : null} />
    </Stack>
  </Screen>;
}
```

Wait for controller readiness before starting it. Ignore permission results after the effect ends; the hook owns controller cleanup when its screen is hidden or removed. The example does not retain the last note through silence—that is a separate presentation choice.

## Android and LightOS

An app without LightOS support uses Android permission prompts, even if a LightOS emulator host is installed. An app with LightOS support uses host permissions when that host is available. The LightOS emulator's location permission is handled by Android because the host does not provide it.

Host permissions and controller readiness are different things. If a host configuration fails, do not describe it as a microphone hardware failure. See [LightOS setup](/light-sdk) for host-specific requirements.

## Check each outcome

Try granting permission, denying it, returning from another screen, and closing the app while a request is pending. A passing `ink check` validates code and configuration; it cannot grant permission or verify a sensor.
