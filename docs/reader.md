---
title: "Reader"
description: "Present long-form and paginated EPUB, text, and PDF documents."
tag: "Planned"
---

`@ink/reader` opens EPUB, plain-text, and PDF files in one native reading session. Opening once produces document information, progress, navigation, appearance controls, and the view.

## Open a document

Create a reader from a temporary file handle or durable file reference:

```tsx
const book = reader(bookFile, {
  flow: "paginated",
  appearance: {
    textScale: 1.1,
    lineHeight: "relaxed",
  },
  progress: localReaderProgress("reader.book-42"),
});

{book.phase === "opening" ? <Text>Opening book</Text> : null}
{book.phase === "ready" ? (
  <ReaderView session={book} accessibilityLabel={book.info.title} />
) : null}
```

The ready snapshot contains `info`, `progress`, and appearance. `info` contains a content fingerprint, format, title, authors, language, and table-of-contents sections. Reader does not parse the same file through a separate metadata resource.

EPUB and text reflow to the view and appearance. PDF keeps fixed page layout. EPUB scripts and remote resources do not run.

## Navigate

The session provides ordered `next()`, `previous()`, and `goTo()` commands. `goTo()` accepts a section ID, approximate fraction, or opaque locator. A newer navigation replaces one that has not settled.

Progress contains a versioned locator, approximate fraction, section, optional page information, and update time. Store or synchronise the locator as a complete string; do not parse or construct it.

## Choose progress storage

Use `localReaderProgress(key)` for automatic app-local persistence. Pass `initial` when a remote value should seed a document with no local position.

Provider modules can supply another typed progress adapter that owns reconciliation and synchronisation:

```tsx
const book = reader(bookFile, {
  progress: ReadingAccount.progress(documentId),
});
```

Reader reports position changes to the adapter after page turns, settled scrolling, and backgrounding. The adapter—not Reader—decides whether local or remote progress wins.

Omit `progress` to retain position only for the session lifetime.

## Change appearance

Call `setAppearance()` with text scale, line height, margins, and theme. Changing appearance keeps the semantic locator while repaginating.

`ReaderView` fills available width and height unless explicit dimensions are supplied. Page-turn and scroll gestures have equivalent commands.

## Lifecycle, errors, and accessibility

The session acquires a file lease while open. Leaving the screen closes the document; backgrounding saves progress. A durable reference can be reopened after process death.

Errors distinguish missing or expired files, unsupported formats, invalid or unsafe documents, missing embedded resources, rendering, storage, invalid locators, progress adapter failures, and unexpected failures. Every error has `kind`, `message`, `retryable`, and `operation`.

The native semantic tree exposes headings, paragraphs, lists, links, quotations, page boundaries, and reading order when the document provides them. PDFs without usable text are announced as image-only pages. Focus returns to the nearest text block after repagination.
