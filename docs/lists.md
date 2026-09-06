# Lists

Use `itemHeight` for fixed-height rows, or `estimatedItemHeight` when content determines height. Both values are Ink logical units. `gap` supplies normal space between rows without making the app add it to each measurement.

```tsx
<List
  items={messages}
  estimatedItemHeight={120}
  gap={47}
  keyExtractor={message => message.id}
  renderItem={message => <Text size={18}>{message.text}</Text>}
/>
```

Native layout measures mounted rows and uses estimates for the rest. A prefix-offset index finds the viewport window; one viewport before and after the visible area is mounted. Native scrolling remains responsive while JavaScript works, but rows outside the mounted window can briefly be blank until JavaScript catches up. Row-local React state disappears when a row unmounts; keep durable state in the app.

The visible item key and its offset are preserved when earlier rows are inserted, removed or remeasured. If that item is removed, the row at its old index becomes the anchor (or the last remaining row). `followEnd` follows additions only when already within 64 logical units of the end; otherwise the reader's anchor is retained.

Use immutable item arrays and replace changed items when changing content. Currently native tree reconstruction and offset rebuilding are linear in the number of keys. Fixed-height rows skip row measurement. Further optimisation should be driven by measurements.

Only ordered keys and layout metadata cross into native code; item objects remain in React. Key and content-version metadata is bounded to 128 KiB of UTF-8 JSON per list, and oversized metadata produces a clear error. Use compact stable keys or bound the loaded history. This limit keeps metadata below the runtime message limit; very large datasets need a paged model rather than silently dropping commits. Window events carry the data revision and obsolete revisions are ignored.

The template's Variable-height List is a local fixture for expanding rows and inserting content. It is not a real-app workflow validation.

Measurements are cached by stable item key, content version and width. Ink keeps both the full and scrollbar-adjusted widths across layout passes. Replace changed items immutably so only their cached measurements are invalidated; prepend and reorder operations retain measurements for unchanged items. When an external value changes row geometry (for example typography or density), change `measurementKey` to invalidate offscreen measurements as well. Mounted rows are remeasured during native layout, including completed image loads.

The mounted window follows its existing first key immediately when items are prepended, preserving React identity for retained rows. If multiple lists are visible, the native anchor nearest the viewport top is used deterministically.
