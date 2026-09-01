---
title: "Reader"
description: "Present long-form and paginated EPUB, text, and PDF documents."
---

`@ink/reader` opens EPUB, plain-text, and PDF files in a native reading view. It provides document structure, page or scroll navigation, appearance controls, and resumable reading progress.

## Open a document

Create a reader from a `FileHandle` and pass it to `ReaderView`:

```tsx
import { ReaderView, reader } from "@ink/reader";
import { Text, match } from "ink";

const book = reader(bookFile, {
  flow: "paginated",
  appearance: {
    textScale: 1.1,
    lineHeight: "relaxed",
  },
  progress: {
    key: "reader.the-left-hand-of-darkness",
  },
});

{match(book, {
  opening: () => <Text>Opening book</Text>,
  ready: () => (
    <ReaderView
      reader={book}
      accessibilityLabel="The Left Hand of Darkness"
    />
  ),
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

The file handle can come from `@ink/files` or another Ink package. It has no filesystem path or Android URI.

`flow` is `"paginated"` by default or `"scroll"` for a continuous vertical document. EPUB and text reflow to the current width and appearance. PDF keeps its fixed page layout; its flow controls whether pages turn one at a time or continue vertically.

## Read document information

Use `readerInfo()` when you need metadata before opening the reading view:

```tsx
import { readerInfo } from "@ink/reader";
import { Text, match } from "ink";

const info = readerInfo(bookFile);

{match(info, {
  loading: () => <Text>Reading book details</Text>,
  ready: (result) => <Text>{result.value.title}</Text>,
  error: (result) => <Text>{result.error.message}</Text>,
})}
```

The ready value contains:

| Field | Value |
| --- | --- |
| `fingerprint` | Stable content identity used to validate progress. |
| `format` | `"epub"`, `"text"`, or `"pdf"`. |
| `title` | Document title or a normalised fallback. |
| `authors` | Available author names. |
| `language` | Document language, or `null`. |
| `sections` | Table-of-contents entries with `id`, `title`, and `depth`. |

## Navigate

The reader controller provides these actions:

- `next()` moves one page or viewport forwards.
- `previous()` moves one page or viewport backwards.
- `goTo({ sectionId })` opens a section from `info.sections`.
- `goTo({ fraction })` moves to an approximate position from 0 to 1.
- `goTo({ locator })` restores an exact semantic position when it still resolves.

Calls at the start or end do nothing. A newer `goTo()` replaces navigation that has not settled yet.

The current `progress` contains:

| Field | Value |
| --- | --- |
| `locator` | An opaque, versioned position for this document fingerprint. |
| `fraction` | Approximate position from 0 to 1. |
| `sectionId`, `sectionTitle` | Current section when available. |
| `page`, `pageCount` | Current PDF or paginated-layout page, otherwise `null`. |
| `updatedAtMs` | Unix time when the position last changed. |

Store or synchronise `locator` as a complete string. Do not parse or construct it.

## Resume reading

Pass a stable progress key to save the reading position automatically:

```tsx
const book = reader(bookFile, {
  progress: {
    key: "reader.book-42",
    initialLocator: remoteProgress,
  },
});
```

A saved locator for the same fingerprint takes precedence over `initialLocator`. The initial locator is used when no matching local progress exists. Invalid saved progress falls back to the beginning.

Keys must match `[A-Za-z0-9][A-Za-z0-9._-]{0,127}`. Progress is saved after a page turn, after scrolling settles, and when the app enters the background.

Omit `progress` to keep the reading position only for the controller lifetime. Use `book.progress.locator` when your app synchronises progress elsewhere.

## Change the appearance

Set initial appearance in `reader()` or change it with `setAppearance()`:

```tsx
import { Button } from "ink";

<Button onPress={() => book.setAppearance({
  textScale: 1.25,
  lineHeight: "relaxed",
  margins: "wide",
  theme: "dark",
})}>
  Use large text
</Button>
```

| Option | Values | Default |
| --- | --- | --- |
| `textScale` | 0.8 to 2 | 1 |
| `lineHeight` | `"compact"`, `"standard"`, `"relaxed"` | `"standard"` |
| `margins` | `"narrow"`, `"standard"`, `"wide"` | `"standard"` |
| `theme` | `"light"`, `"dark"`, `"system"` | `"system"` |

Changing appearance keeps the current semantic position while the document repaginates.

## Lifecycle and errors

The reader opens while its screen is active. Leaving the screen closes the document. Moving the app to the background saves progress. Returning to the same screen restores the current locator.

The package requests no runtime permission. It reads only a `FileHandle` already available to the app. EPUB scripts and remote document resources do not run.

Errors distinguish missing files, unsupported formats, invalid or unsafe documents, missing embedded resources, storage, rendering, invalid locators, and unexpected failures. Every error has `kind`, `message`, `retryable`, and the failed operation where available.

## Accessibility

`ReaderView` exposes headings, paragraphs, lists, links, quotations, page boundaries, and reading order when the document provides them. PDFs without usable text are announced as image-only pages.

Page-turn and scroll gestures have equivalent `next()` and `previous()` actions. Text scaling keeps the current locator, and focus returns to the nearest text block after repagination.

Give `ReaderView` an `accessibilityLabel` that names the document. This label does not replace the document's title and structure.
