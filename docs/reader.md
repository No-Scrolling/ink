---
title: "Reader"
description: "Native document reading with saved positions."
tag: "Planned"
---

> **Planned.** This package is not implemented. The APIs below describe the proposed design.

`@ink/reader` presents supported documents with native pagination, text layout, selection and zoom. Use it for saved articles, manuals, books and PDF tickets.

```tsx
import { Screen } from "ink";
import { Reader, useReader } from "@ink/reader";

export function DocumentScreen({ fileId }: { fileId: string }) {
  const reader = useReader({ fileId });
  return <Screen title="Document"><Reader controller={reader} /></Screen>;
}
```

The controller opens after commit while visible. Its state includes loading, ready or error, with current location and navigation availability. `goTo(location)` and search operations return promises. The native view owns reading gestures; it does not send the document text through JavaScript for each frame.

## Positions

Persist a format-specific locator with the document ID and revision. A PDF page number, an EPUB locator and an article anchor are different shapes. If content changes, resolve the locator or fall back visibly; a character offset is not universally stable.

Checkpoint after meaningful navigation and on leaving, rather than writing on every scroll update. Release the reader attachment when hidden. Reopening uses the durable file and saved locator, not an old native handle.

## Formats

Register the format capabilities the app needs, such as PDF, EPUB or structured articles. Supported features are documented per decoder. Encrypted documents, DRM and embedded scripting are not implied by supporting the container format.

An RSS or extraction package can produce an article model in TypeScript. Remote HTML must be sanitised and converted into supported content; the reader does not execute web scripts. Links are explicit actions through [System](system.md).

Use [Downloads](downloads.md) to save content and [Records](records.md) for the library, annotations and reading progress. Search large documents natively with cancellation and bounded result pages.
