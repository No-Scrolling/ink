---
title: "Store"
description: "Persist small settings with explicit decoding and migrations."
tag: "In development"
---

> **In development.** SQLite persistence, revision-checked updates, decoding, migrations and snapshots across runtimes and processes are implemented. Corruption, migration and contention scenarios need broader verification.

`@ink/store` stores small JSON values: selected units, saved locations, a sort order or the last selected account. Use [Records](records.md) for indexed collections and [Secure store](secure-store.md) for credentials.

Committed writes notify active observers in other app runtimes through private Android broadcasts. Observers reload the changed key; a newly started process reads SQLite directly.

```ts
import { createStore } from "@ink/store";

type Settings = { units: "celsius" | "fahrenheit" };

export const settings = createStore<Settings>({
  key: "weather.settings",
  version: 1,
  initial: { units: "celsius" },
  decode(value: unknown): Settings {
    if (typeof value !== "object" || value === null || !("units" in value)) {
      throw new Error("Invalid settings");
    }
    if (value.units !== "celsius" && value.units !== "fahrenheit") {
      throw new Error("Invalid temperature units");
    }
    return { units: value.units };
  },
});

export async function setUnits(units: Settings["units"]) {
  await settings.update(value => ({ ...value, units }));
}
```

Creating a store declares it without reading disk. `get()` loads and decodes its value. `set(value)` resolves after an atomic persisted replacement. `update(transform)` uses revision-checked updates; its synchronous, pure transform may run again if another writer wins. Do not put side effects inside it.

## Observe a setting

```tsx
import { Text, useSnapshot } from "ink";
import { settings } from "./settings";

export function Units() {
  const value = useSnapshot(settings);
  if (value.status === "loading") return <Text>Loading settings</Text>;
  if (value.status === "error") return <Text>{value.error.message}</Text>;
  return <Text>{value.data.units}</Text>;
}
```

Observation activates loading after commit. Snapshots are `loading`, `ready` with `data`, or `error`. `get()` can be used by a worker without React. Successful local writes and committed writes from another app runtime invalidate observations. UI commands should use `useAction` to display write failures.

## Stored shape

Only JSON data is persisted. Decode on reading; changing a TypeScript type does not migrate existing data. When increasing `version`, supply `migrate(oldValue, oldVersion)` returning the new JSON value, then validate it with `decode`. Migration and replacement are atomic for this key.

A corrupt or unsupported newer value produces a recoverable error; it is not silently replaced with defaults. Defaults apply when the key does not exist. `reset()` explicitly restores the initial value. Keys are app-wide names, not access-control boundaries.
