---
title: "Save and load data"
description: "Choose temporary state, saved preferences or fetched data."
---

Different values need different lifetimes:

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

Tap **Add one**, close the app and reopen it. The count should remain. `useSnapshot` updates the screen when saved data changes; `useAction` tracks a command and handles its rejected promise.

## Fetch data

Use a [resource](/data) when switching pages should reuse a previous response. It holds data in memory, not permanent storage. A failed refresh can leave useful cached content visible; show its `refreshError` and offer `source.refresh` as a retry.

Import `@ink/network` before using `fetch`. See [network requests](/network) for the supported API and [resources](/data) for a complete screen example.

**Next:** [build and install a release](/release).
