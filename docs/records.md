---
title: "Records"
description: "SQLite for offline collections, search and durable outboxes."
tag: "Planned"
---

> **Planned.** This package is not implemented. The APIs below describe the proposed design.

`@ink/records` gives domain modules a native SQLite database. Use it for chat history, passes, downloaded libraries, feed entries or transit stops. Screens call domain functions and observe results; SQL stays close to the data it describes.

```ts
import { openDatabase } from "@ink/records";

export const database = openDatabase({
  name: "messages",
  migrations: [{
    version: 1,
    sql: `
      CREATE TABLE messages (
        id TEXT PRIMARY KEY, room_id TEXT NOT NULL,
        body TEXT NOT NULL, created_at INTEGER NOT NULL
      );
      CREATE INDEX messages_room_time
        ON messages(room_id, created_at DESC, id DESC);
      CREATE TABLE outbox (
        id TEXT PRIMARY KEY, payload TEXT NOT NULL
      );
    `,
  }],
});

export async function recentMessages(roomId: string) {
  const db = await database;
  return db.query(
    `SELECT id, body, created_at FROM messages
     WHERE room_id = ? ORDER BY created_at DESC, id DESC LIMIT ?`,
    [roomId, 40],
    { decode: decodeMessageRow },
  );
}
```

The example supplies `decodeMessageRow` from the app's domain module. Query results begin as unknown row values; a decoder establishes the return type. Parameters bind values, never table names or SQL fragments. Keep ordering deterministic and use keyset cursors for growing histories.

## Transactions

```ts
export async function queueMessage(id: string, roomId: string, body: string) {
  const db = await database;
  await db.transaction(async tx => {
    await tx.execute(
      "INSERT INTO messages (id, room_id, body, created_at) VALUES (?, ?, ?, ?)",
      [id, roomId, body, Date.now()],
    );
    await tx.execute("INSERT INTO outbox (id, payload) VALUES (?, ?)",
      [id, JSON.stringify({ id, roomId, body })]);
  });
}
```

Commit makes both writes durable; an exception rolls both back. Keep transactions short and await every operation through `tx`. Do not perform network calls, wait for user input or use the outer database inside a transaction. The native connection queue serialises writers across foreground and headless runtimes.

A worker later sends the outbox entry using its ID as a provider idempotency key, then reconciles the acknowledgement. SQLite cannot make a remote request atomic with a local commit. See [Background](background.md).

## Live queries

`db.watch(sql, parameters, { decode, tables })` creates a readable source for `useSnapshot`. Declare the tables that invalidate the query. Observation starts after commit; changes rerun the query and publish an immutable `loading`, `ready` or `error` snapshot. Coalesce repeated invalidations and keep the result bounded.

A list displaying forty rows should not load ten thousand records to slice them in JavaScript. Filter, sort, join and aggregate in SQLite. Use a native full-text index for larger search corpora when that capability is linked.

## Files and migrations

Store metadata and file IDs here; keep artwork, photos and audio in [Files](files.md). Deleting a row does not automatically delete a file shared by other records. A domain module owns reference tracking and cleanup.

Migrations run once, in ascending versions, inside transactions before exposing the database. A failed migration preserves the previous database and surfaces an error. Older app versions refuse a newer schema. A bundled read-only database can seed a dictionary or transit catalogue; copy it into managed storage before applying writes.

Database handles are runtime-local and expose `close()`. Worker runtimes release their handles on completion. Persist record IDs, never handles. App-private storage is not encrypted credential storage, and uninstalling the app removes it.
