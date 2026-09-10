---
title: "Save a setting"
description: "Edit a value, save it and return to the previous page."
---

Keep an unfinished edit in React state. Save it before navigating back so the previous page shows the updated value.

This example uses a [store](/store) exported from `lib/preferences.ts` with a `referenceHz` number:

```tsx
import { useState } from "react";
import { back, ErrorState, LoadingState, Screen, Text, useAction, useSnapshot } from "ink";
import { TextInput } from "ink/input/numeric";
import { preferences } from "../lib/preferences";

export default function ReferencePitch() {
  const saved = useSnapshot(preferences);
  const retry = useAction(preferences.get);
  if (saved.status === "loading") return <Screen title="Reference pitch"><LoadingState /></Screen>;
  if (saved.status === "error") return <Screen title="Reference pitch"><ErrorState message={saved.error.message} onRetry={retry.run} /></Screen>;
  return <Editor initial={saved.data.referenceHz} />;
}

function Editor({ initial }: { initial: number }) {
  const [draft, setDraft] = useState(String(initial));
  const save = useAction(async (value: string) => {
    const referenceHz = Number(value);
    if (!Number.isInteger(referenceHz) || referenceHz < 400 || referenceHz > 480) {
      throw new Error("Enter a frequency from 400 to 480 Hz.");
    }
    await preferences.update(current => ({ ...current, referenceHz }));
    back();
  });
  return <Screen title="Reference pitch">
    <TextInput value={draft} onChange={setDraft} suffix="Hz" autoFocus onSubmit={save.run} />
    {save.status === "error" && <Text>{save.error.message}</Text>}
  </Screen>;
}
```

This example accepts 400–480 Hz. Change the range for your app. The keyboard's **Done** action saves the edit; the header's **Back** action leaves without saving. Keep inputs on their own screen, with only errors or hints beneath them.

`useAction` ignores repeated submissions while saving and makes errors available to display below the input. Keep the input visible while the save is pending. For a toggle that saves immediately, wrap the store update in an action and disable the toggle while pending.

`ReferencePitch` loads the saved value before rendering `Editor`. Use the same structure when a component needs loaded data before it can run its hooks.
