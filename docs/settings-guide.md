---
title: "Save a setting and go back"
description: "Keep editing local, then finish saving before navigating."
---

A setting usually has two values: the saved value and the draft the person is editing. Keep the draft in React state. Save it before returning so the previous page immediately shows the new value.

Suppose `lib/preferences.ts` exports a Store with a `referenceHz` number. This page is a complete editing flow using that store:

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

The range is this app's policy, not an Ink restriction. Numeric inputs default to a Done action. The keyboard confirms the edit; the header's Back action leaves without saving. Keep inputs on their own screen, with only errors or hints beneath them.

`useAction` ignores overlapping runs and handles failures. Do not add a separate saving page or change the input's layout while a save is pending. For toggles that save immediately, use an action around the Store update and disable the toggle while pending.

This parent/child split also works when a component needs a non-null location or loaded account before subscribing to a resource. You do not need an artificial empty data source.

See [Store](/store) for defining and validating preferences and [navigation](/navigation) for shared providers.
