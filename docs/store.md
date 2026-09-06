---
title: "Store"
description: "Persist small settings with explicit decoding and migrations."
---

`@ink/store` stores small JSON values: selected units, saved locations, a sort order or the last selected account. Use [Secure store](secure-store.md) for credentials.

## JSON storage

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

## Read-only SQLite access

Import a bundled `.db` asset and open it with `openDatabase(asset)`. Query it with `db.query(sql, parameters, { signal })` and release the runtime-local handle with `db.close()`.

```ts
import { openDatabase } from "@ink/store";
import stops from "./stops.db";

const database = await openDatabase(stops);
try {
  const rows = await database.query(
    "SELECT id, name FROM stops WHERE name LIKE ? ORDER BY name LIMIT ?",
    ["Central%", 20],
  );
  // Decode rows into the app's stop type.
} finally {
  await database.close();
}
```

Queries start with `SELECT`, `WITH` or `EXPLAIN`; the database also enforces read-only access. Parameters bind strings, finite numbers or null without SQL interpolation. Rows contain strings, numbers or null; binary columns and integers outside JavaScript's safe range reject. Cast large integers to text when necessary. A runtime can open eight handles. Results are limited to 10,000 rows and approximately 400 kB of encoded row data; use SQL limits and pagination for larger collections. An abort signal cancels a running query.

Buses' stops database is the initial use case. The bundled database is read-only; an app update replaces its asset rather than migrating a writable copy. Query results are arrays of row values that the app decodes into its own types. SQL values use bound parameters. The app owns search terms and result limits; filtering and ordering stay in SQLite.

No ORM, live-query system, outbox abstraction or general schema-management layer is added. Writable databases and transactions can be considered when a real migration needs them. The existing JSON interface remains the simple choice for settings and small collections.
