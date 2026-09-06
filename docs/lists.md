# Lists

Every `List` is virtualised and sizes rows from their content. Ink estimates unseen rows from the measured rows, using an internal starting estimate until measurements arrive. No height configuration is needed. `gap` supplies normal space between rows without making the app add it to each measurement.

```tsx
<List
  items={messages}
  gap={47}
  keyExtractor={message => message.id}
  renderItem={message => <Text size={18}>{message.text}</Text>}
/>
```

Native layout measures mounted rows and uses estimates for the rest. A prefix-offset index finds the viewport window; one viewport before and after the visible area is mounted. Android presents ready content before processing the next drag movement and retains the last complete frame while destination rows load. Under load, the displayed content and scrollbar can briefly trail the requested position. Row-local React state disappears when a row unmounts; keep durable state in the app.

The visible item key and its offset are preserved when earlier rows are inserted, removed or remeasured, including during an active drag. If that item is removed, the row at its old index becomes the anchor (or the last remaining row). `followEnd` follows additions only when already within 64 logical units of the end and no touch gesture is active; otherwise the reader's anchor is retained.

Use immutable item arrays and replace changed items when changing content. Keys are cached while the item array and key extractor are unchanged. Window-only commits reuse native key and content-version metadata instead of sending the entire dataset again.

Native layout maintains a height index for each cached width. Updating a measured height and finding a row offset take logarithmic time; changing the estimate for unseen rows does not rebuild all offsets. Dataset revisions and new widths still require linear index preparation. Layout reads offsets only for mounted rows, and an unchanged anchor keeps its existing index.

Pending viewport requests are coalesced per list before JavaScript receives them, so dragging the scrollbar thumb replaces an undelivered destination with the latest one. This reduces catch-up work but does not cancel a React render already in progress or make an arbitrary destination available immediately.

Only ordered keys and layout metadata cross into native code; item objects remain in React. Key and content-version metadata is bounded to 128 KiB of UTF-8 JSON per list, and oversized metadata produces a clear error. Use compact stable keys or bound the loaded history. This limit keeps metadata below the runtime message limit; very large datasets need a paged model rather than silently dropping commits. Window events carry the data revision and obsolete revisions are ignored.

The template's Follow new items page keeps Add and Clear actions in the screen's pinned `header` while the list scrolls below. Following is always enabled in this example. The Virtualised List page demonstrates 5,000 automatically sized rows. These are local fixtures, not real-app workflow validation.

Measurements are cached by stable item key, content version and width. Ink keeps both the full and scrollbar-adjusted widths across layout passes. Replace changed items immutably so only their cached measurements are invalidated; prepend and reorder operations retain measurements for unchanged items. When an external value changes row geometry (for example typography or density), change `measurementKey` to invalidate offscreen measurements as well. Mounted rows are remeasured during native layout, including completed image loads.

The mounted window follows its existing first key immediately when items are prepended, preserving React identity for retained rows. If multiple lists are visible, the native anchor nearest the viewport top is used deterministically.

## Automatic pagination

Provide `onLoadMore: () => Promise<void>` and `hasMore` for a paged data source. Ink calls the callback when the native viewport's forward overscan reaches the last loaded row (roughly one viewport ahead). The initial JavaScript window does not trigger requests before native layout. Short or empty lists can request enough content to fill that window.

Append the fetched items immutably and set `hasMore` to false at the end. The app owns its cursor, fetching and data; Ink owns when to request more. Only one request runs at a time, and an unchanged item boundary is not requested repeatedly. A rejected request displays its error with a retry action; normal loading adds no spinner or button. A successful request that adds no rows is not repeated for the same boundary. Use a new List key when replacing a query/data source.

The Pagination example simulates 20-item pages up to 100 items. It contains no manual load-more control.

For history, provide `onLoadOlder` and `hasOlder`, then prepend fetched items immutably. Ink requests older rows when the backward overscan reaches the first loaded item. Requests are sequential and failures expose retry. `initialEnd` starts the mounted window at the last rows; it does not independently scroll a containing Screen. ConversationScreen owns both the initial bottom position and subsequent reply/send scroll requests. All lists use the containing screen's scrolling, rather than independent nested scroll containers.
