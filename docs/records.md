---
title: "Records"
description: "Define typed, queryable local collections and bundled reference data."
tag: "Planned"
---

`@ink/records` stores large collections that apps need to query, page, and update one record at a time. It is designed primarily for local and third-party module authors. Screens usually consume domain operations such as `Messaging.thread()` or `Transit.searchStops()` instead of creating database queries.

Records has two interfaces:

- `defineRecords()` for mutable app-owned data;
- `defineReferenceData()` for immutable indexed data bundled with a module.

Ink uses SQLite in the Android implementation and an in-memory adapter in preview. SQLite connections, database files, and SQL strings are not part of either interface.

## Choose Records or Store

Use Store when the app can load and replace one complete value, such as settings, a short favourites list, or saved locations.

Use Records when the app needs to:

- keep hundreds or thousands of independently updated items;
- load a collection in pages;
- filter, search, or sort without loading everything;
- update one item without rewriting the complete collection;
- save several related changes atomically;
- keep an outbox for background synchronisation.

Common uses include messages, email, feeds, downloaded media metadata, inventory, offline forms, financial transactions, fitness history, and searchable catalogues. Files owns attachment and media bytes; Records stores only their durable references and metadata.

## Common uses

| Domain module | Example operation | Why it needs Records |
| --- | --- | --- |
| Messaging | `Messaging.thread(id)` | Paginated history, unread state, pending messages, deduplication, and an outbox. |
| Mail | `Mail.mailbox("inbox")` | Mailbox filters, drafts, flags, offline actions, and sync cursors. |
| Feed or read-later | `Feed.unread()` | Thousands of ordered articles updated individually and loaded in pages. |
| Podcasts or music | `Library.downloaded({ unplayed: true })` | Tracks belong to several collections and keep independent download and playback state. |
| Inventory or field forms | `Inventory.pendingUploads()` | Searchable records, offline edits, and outgoing work saved together. |
| Finance | `Budget.transactions({ month })` | Date and category queries, imports, deduplication, and transfers. |
| Fitness or medication | `Health.history(range)` | Append-heavy history queried by date without loading every entry. |
| Transit reference data | `Transit.searchStops(text)` | Hundreds of thousands of indexed records searched without loading the dataset. |
| Dictionary or field guide | `Guide.search(text)` | Prefix or text lookup over a large bundled catalogue. |

Small task lists, passes, settings, and short collections remain Store use cases unless they gain paging, indexed search, or offline synchronisation requirements.

## Define mutable records

Use `defineRecords()` inside a local or published module:

```tsx
import {
  collection,
  defineRecords,
  index,
  migration,
} from "@ink/records";

type Conversation = {
  id: string;
  title: string;
  updatedAtMs: number;
  unread: boolean;
};

type Message = {
  id: string;
  conversationId: string;
  text: string;
  sentAtMs: number;
  delivery: "pending" | "sent" | "failed";
};

type OutboxItem = {
  id: string;
  kind: "send-message";
  createdAtMs: number;
};

const messagingCollections = {
  conversations: collection<Conversation>({
    key: "id",
    indexes: [index(["unread", "updatedAtMs"])],
  }),
  messages: collection<Message>({
    key: "id",
    indexes: [
      index(["conversationId", "sentAtMs"]),
      index(["delivery", "sentAtMs"]),
    ],
  }),
  outbox: collection<OutboxItem>({
    key: "id",
    indexes: [index(["createdAtMs"])],
  }),
};

const messagingMigrations = {
  1: migration(({ collections }) => [
    collections.messages.addField("delivery", "sent"),
    collections.messages.addIndex(["delivery", "sentAtMs"]),
  ]),
};

const messaging = defineRecords({
  key: "messaging",
  version: 2,
  collections: messagingCollections,
  migrations: messagingMigrations,
});
```

Every record has one stable string or integer key. Indexes are declared with the schema so Ink can validate and plan every reachable query during compilation.

Package record keys are automatically namespaced. Collections cannot be opened by another package unless the owning package exports a typed grant.

## Declare a query

Queries are named, static module definitions. Their inputs can vary at runtime, but their collection, filters, ordering, and maximum page size cannot.

```tsx
const threadMessages = messaging.query<{ conversationId: string }>({
  collection: "messages",
  where: ({ record, input }) =>
    record.conversationId === input.conversationId,
  orderBy: [
    { field: "sentAtMs", direction: "descending" },
    { field: "id", direction: "descending" },
  ],
  pageSize: 50,
});

export function thread(conversationId: string) {
  return threadMessages({ conversationId });
}
```

The predicate uses Ink's pure-expression subset. The compiler rejects a filter or ordering that cannot use a declared key or index.

A query is a resource with `loading`, `ready`, and `error`. Its ready value contains `items`, `hasMore`, and `activity`. Call `loadMore()` for the next cursor page or `reload()` to replace the loaded pages.

Queries refresh automatically after a committed write that can affect them. They preserve stable ordering by requiring a unique key as the final ordering field.

## Expose a domain interface

Screens should use the operation that describes their task:

```tsx
const messages = Messaging.thread(conversationId);

{match(messages, {
  loading: () => <Text>Loading messages</Text>,
  ready: ({ value }) => value.items.map((message) => (
    <Text>{message.text}</Text>
  )),
  error: ({ error }) => <Text>{error.message}</Text>,
})}

{messages.status === "ready" && messages.value.hasMore ? (
  <Button onPress={() => messages.loadMore()}>Load older messages</Button>
) : null}
```

This keeps indexes, storage versions, pagination cursors, and record layout behind the module interface. An app that does not need a published package can define the same small domain operations in a local module.

## Write records atomically

Use a transaction definition for one or more related changes:

```tsx
type SendMessage = {
  id: string;
  conversationId: string;
  text: string;
  createdAtMs: number;
};

const enqueueMessage = messaging.transaction<SendMessage>(({ input, records }) => [
  records.messages.put({
    id: input.id,
    conversationId: input.conversationId,
    text: input.text,
    sentAtMs: input.createdAtMs,
    delivery: "pending",
  }),
  records.outbox.put({
    id: input.id,
    kind: "send-message",
    createdAtMs: input.createdAtMs,
  }),
  records.conversations.update(input.conversationId, {
    updatedAtMs: input.createdAtMs,
  }),
]);
```

The complete transaction commits or none of it does. `put()` inserts or replaces by key. `insert()` fails when the key exists. `update()` fails when it does not. `remove()` deletes by key.

A transaction definition creates an action with `idle`, `running`, `success`, and `error`. Transaction expressions use the pure subset and cannot perform network, file, clock, or native work while the transaction is open.

Declare an outbox as an ordinary collection. The domain module decides what an outgoing operation means, how to retry it, and how to resolve a conflict. Records provides atomic storage, not automatic synchronisation.

## Migrate a collection

Increase the record group version when stored shapes or indexes change. The current schema above includes `delivery`; `messagingMigrations` describes how an installed version 1 store reaches that schema.

Migrations are static, ordered, and atomic. They can add, remove, rename, or transform fields with the pure subset and can add or remove declared indexes. A failed migration leaves the previous version untouched and returns a `migration` error.

The compiler checks that every stored version has a complete path to the current schema.

## Use Records in background work

A domain module can grant a Background plan access to named queries and transactions. It does not pass a database connection or unrestricted collection access.

For example, a messaging sync plan can read the declared `pendingMessages` query and invoke the declared `markSent` transaction. Background owns scheduling and retries; Network owns remote requests; the messaging module owns deduplication and conflict policy.

Only application-scoped record groups can enter background work. Temporary handles and live screen-state references cannot be stored in records or captured by a plan. Materialise permitted scalar and object inputs when an action starts.

## Define bundled reference data

Use `defineReferenceData()` for a large immutable dataset shipped with a module:

```tsx
import {
  defineReferenceData,
  index,
  textIndex,
} from "@ink/records";

type Stop = {
  atcoCode: string;
  name: string;
  locality: string;
  indicator: string;
};

const stops = defineReferenceData<Stop>({
  key: "transit.stops",
  source: "./data/stops.csv",
  recordKey: "atcoCode",
  indexes: [
    index(["atcoCode"]),
    textIndex("search", ["name", "locality"]),
  ],
});
```

`ink package build` validates the source and compiles it into an indexed runtime artefact. The original CSV or JSON source is not included in the APK.

Declare reference queries with key, equality, range, prefix, or text indexes:

```tsx
export const searchStops = stops.query<{ text: string }>({
  search: {
    index: "search",
    text: ({ input }) => input.text,
  },
  orderBy: "relevance",
  limit: 50,
});
```

Reference data supports lookup, filtering, stable ordering, and bounded results. It has no app-facing insert, update, transaction, or migration operations. An app update replaces the complete artefact atomically when its declared version changes.

This interface suits transit stops, timetables, dictionaries, field guides, and large catalogues. A specialised domain module still exposes operations such as `Transit.searchStops()` rather than the generic query definition.

## Preview record modules

Provide small deterministic fixtures for each public query:

```tsx
import { defineRecordsPreview } from "@ink/records/preview";

export default defineRecordsPreview(messaging, {
  collections: {
    conversations: previewConversations,
    messages: previewMessages,
    outbox: [],
  },
});
```

Preview uses an in-memory adapter that satisfies the same query and transaction interface. Include empty, paginated, migration-failed, quota, and transaction-failed scenarios where the domain module exposes those states.

Reference data previews use a small fixture instead of loading the production artefact.

## Limits and unsupported features

One query page contains 1 to 200 records. A transaction can change up to 1,000 records and must fit within its execution timeout. Records must contain serialisable Ink values or explicitly supported durable references; store large text, documents, images, audio, and video in Files.

Records does not support:

- raw SQL or public database connections;
- runtime-created collections or indexes;
- unrestricted joins or cross-package queries;
- general grouping, analytics, geospatial, or vector queries;
- arbitrary JavaScript predicates or transaction callbacks;
- automatic remote synchronisation or universal conflict resolution;
- HTTP response caching;
- secret storage or caller-managed encryption;
- high-frequency sensor event storage.

Use IDs and denormalised fields when a domain spans several collections. Add a specialised query capability only after a concrete module cannot express its workflow with key lookup, indexed filtering, ordering, pagination, and text search.

## Lifecycle and errors

Mutable record groups are application-scoped. Queries observe them while their owning screen or application operation is active. Leaving a screen stops observation and releases loaded pages; it does not delete records or roll back an already committed transaction.

Rust owns query resources, ordering, transaction state, and schema validation. The Android adapter owns durable storage and indexed execution. `ink info` reports linked record groups, bundled artefact sizes, mutable storage requirements, indexes, and background grants.

Errors distinguish unavailable storage, invalid or duplicate records, missing records, unsupported queries, migration failures, corrupt data, quota and size limits, transaction failures, invalid durable references, and unexpected failures. Every error provides `kind`, `message`, `retryable`, and `operation`.
