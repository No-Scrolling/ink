---
title: "Lists"
description: "Display scrolling collections and load more items."
---

## Collections

Use `.map()` with stable keys for small collections. For large collections, `List` mounts the visible rows with a viewport of extra rows on either side. Row heights are measured automatically. The default `gap` is 47, matching the spacing between `Screen` children. Set `gap={0}` for rows without gaps, or provide another value for a compact layout. `followEnd` follows additions only near the end. Stable keys preserve the visible scroll anchor.

```tsx
<List
  items={songs}
  keyExtractor={song => song.id}
  renderItem={song => <Button onPress={() => play(song)}>{song.title}</Button>}
/>
```

`List` uses its containing screen’s scroll position and preserves the full collection’s scroll extent. Keys must be stable and unique. Rows outside the window unmount, so keep durable row state in the parent or a store. Replace arrays and changed items instead of mutating them. Ink keeps the visible row in place when earlier rows load or change height.

### Load more items

Provide `onLoadMore: () => Promise<void>` and `hasMore`. Ink calls your function as the reader approaches the bottom, allowing one request at a time. Your app fetches and appends items, keeps the pagination cursor, and sets `hasMore` to false when finished.

Normal loading adds no button or spinner. A failed request shows an error and retry action. Short lists can load more pages to fill the screen; a successful request that adds no rows is not repeated at the same boundary. Give the List a new React `key` when switching queries or data sources.

For older history, use `onLoadOlder` and `hasOlder`, then prepend the fetched items. Ink preserves the reader’s position. `ConversationScreen` handles this pattern for chats.

### Follow new items

Set `followEnd` to follow additions while the reader is near the bottom. Following pauses while they scroll or read earlier items.

For a custom history view, `initialEnd` mounts the last rows first but does not scroll the screen itself. `ConversationScreen` coordinates both behaviours.

### Changes to row layout

Use `measurementKey` when a setting changes row layout without changing the items—for example, `measurementKey={textSize}`. Normal item updates, width changes and image loads do not need it.

Use compact item keys and limit loaded history. List keys and content versions have a combined limit of 128 KiB of UTF-8 JSON; exceeding it reports an error.

