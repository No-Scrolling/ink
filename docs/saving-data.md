---
title: "Save and load data"
description: "Choose temporary state, saved preferences or fetched data."
---

Choose where to keep data based on how your app uses it:

| Need | Use |
| --- | --- |
| A count or unfinished edit on a screen | React `useState` |
| Preferences that survive closing the app | `@ink/store` |
| Data fetched from a server and reused across screens | `resource` and `useSnapshot` |
| A command such as saving or sending | `useAction` |

## Keep a count between sessions

From your app folder:

```sh
bun add @ink/store@0.1.0
```

Create `lib/count.ts`:

```ts
import { createStore } from "@ink/store";

export const count = createStore({
  key: "count",
  version: 1,
  initial: 0,
  decode(value: unknown) {
    if (typeof value !== "number" || !Number.isSafeInteger(value)) {
      throw new Error("The saved count could not be read.");
    }
    return value;
  },
});
```

The decoder checks saved data before your app uses it. A store is defined outside a component so screens share the same source.

Replace `app/index.tsx` with:

```tsx
import { Button, ErrorState, LoadingState, Screen, Text, useAction, useSnapshot } from "ink";
import { count } from "../lib/count";

export default function Home() {
  const value = useSnapshot(count);
  const retry = useAction(count.get);
  const add = useAction(() => count.update(current => current + 1));
  return (
    <Screen title="Saved count">
      {value.status === "loading" ? <LoadingState /> :
        value.status === "error" ? <ErrorState message={value.error.message} onRetry={retry.run} /> : <>
          <Text>{value.data}</Text>
          <Button onPress={add.run} disabled={add.status === "pending"}>Add one</Button>
          {add.status === "error" && <Text>{add.error.message}</Text>}
        </>}
    </Screen>
  );
}
```

Tap **Add one**, close the app and reopen it. The count remains. `useSnapshot` updates the screen when saved data changes. `useAction` tracks saving and exposes errors; this example disables the button while a save is pending.

## Fetch data

Use a [resource](/data) to keep fetched data available when switching pages. Import `@ink/network` before using `fetch`. The [resource example](/data#cached-data) shows how to load data and retry a failed refresh.

**Next:** [build and install a release](/release).
