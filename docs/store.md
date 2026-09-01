---
title: "Store"
description: "Persist typed app values with schema migration and atomic updates."
tag: "Planned"
---

`@ink/store` persists small, non-sensitive app values that need observable loading, migration, or write errors. It does not provide an HTTP cache, file store, or database.

## Persist a value

Use `storedValue<T>()` for a complete serialisable value:

```tsx
import { storedValue } from "@ink/store";
import { Button, Text, match } from "ink";

type Settings = {
  unit: "celsius" | "fahrenheit";
  alerts: boolean;
};

const settings = storedValue<Settings>("settings", {
  initial: { unit: "celsius", alerts: false },
  version: 1,
});

{match(settings, {
  loading: () => <Text>Loading settings</Text>,
  ready: ({ value }) => <Text>{value.unit}</Text>,
  error: ({ error }) => <Text>{error.message}</Text>,
})}

<Button onPress={() => settings.set({
  unit: "fahrenheit",
  alerts: settings.value.alerts,
})}>
  Use Fahrenheit
</Button>
```

`set()` replaces the complete value atomically. `update()` applies an Ink pure function to the latest stored value, which prevents two callers from overwriting unrelated changes. `reset()` restores the declared initial value and removes its saved record.

A ready value has `activity` as `"idle"` or `"saving"` and an optional `warning` when a write failed but the previous value remains usable.

Use core `persistedState()` for simple UI preferences that do not need visible loading, migration, or storage errors.

## Migrate a stored value

Increase `version` when the stored representation changes and provide one migration for each supported previous version:

```tsx
const settings = storedValue<Settings>("settings", {
  initial: { unit: "celsius", alerts: false },
  version: 2,
  migrations: {
    1: (old: V1Settings) => ({ unit: old.unit, alerts: false }),
  },
});
```

Migrations use Ink's pure-expression subset and run in order before the value becomes ready. A missing or failed migration returns a `migration` error and leaves the original record untouched. Pass `onIncompatible: "reset"` only when losing the old value is acceptable.

Package-created keys are automatically scoped to the package. App keys must match `[A-Za-z0-9][A-Za-z0-9._-]{0,127}`. Intentional sharing across packages requires an exported typed `StoreKey<T>` rather than repeating a string.

## Choose the right storage module

Use:

- Network cache options for reproducible HTTP responses;
- Background task results for the latest durable result of scheduled work;
- Secure store for credentials and secret material;
- Files for documents, media, and values larger than 1 MiB;
- [Records](records.md) for large queryable collections and atomic multi-record updates.

Store saves one complete value per key. It is not suitable for message histories, outboxes, offline databases, or collections that must update one record without rewriting the rest.

## Lifecycle and errors

Stored values are application-scoped. Declarations with the same typed key reconnect to one in-memory value. Writes for one key are serialised and failed writes never expose a partial record.

Values are limited to 1 MiB each and share an 8 MiB app quota. Errors distinguish unavailable storage, quota limits, corrupt records, incompatible schemas, failed migrations, storage failures, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.
