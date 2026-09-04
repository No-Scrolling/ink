---
title: "Files"
description: "Managed files, imports and large content without JavaScript copies."
tag: "Design specification"
---

`@ink/files` manages app documents, cache files and user-selected imports. A `FileRef` refers to native content; its `id` can be persisted. Opening that ID later can fail if the file was deleted or its external permission expired.

```ts
import { files } from "@ink/files";

const selected = await files.pick({ types: ["application/pdf"] });
if (selected) {
  const saved = await files.import(selected, { name: "Ticket.pdf" });
  await saveTicket({ fileId: saved.id, name: "Ticket" });
}
```

Here `saveTicket` is the app's record operation. Picking grants access to a selection; importing copies it into managed documents for offline use. The picker survives its external activity round trip. User cancellation returns `null`.

## Ownership

| Storage | Use | Lifetime |
| --- | --- | --- |
| Documents | Tickets, attachments, offline books | Explicit deletion or app-data removal. |
| Cache | Re-creatable artwork and responses | May be evicted. |
| Temporary | Captures and intermediate exports | Short-lived; promote before persisting a reference. |
| External selection | A document owned elsewhere | Provider access may be revoked. |

`files.open(id)` resolves a reference. `stat`, `remove`, `copy` and `export` are asynchronous native operations. Names are display names, not arbitrary absolute filesystem paths. Native boundaries validate IDs and access rights; serialising a reference does not extend its lifetime.

## Read and write

`readText` is useful for a small configuration file; `readBytes` materialises the full content. Use streams for larger content and native file references for images, playback, document reading and uploads. Streams support cancellation and backpressure; callers release them on completion or abort.

Writes use temporary content followed by atomic replacement where supported by the destination. A failed export to an external provider may need user intervention; it is not a database transaction. Check available space before large imports, and handle out-of-space failures during the operation too.

[Downloads](downloads.md) writes into managed storage. Removing a download or record must follow the app's retention policy; merely closing its screen does not delete its content.
