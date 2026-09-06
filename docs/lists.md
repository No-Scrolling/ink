---
title: "Lists"
description: "Display collections with automatic virtualisation, pagination and scrolling."
---

Use `List` for collections that scroll. It measures row heights automatically and mounts only the visible rows plus extra rows on either side. You do not need height hints.

```tsx
<List
  items={messages}
  keyExtractor={message => message.id}
  renderItem={message => <Text size={18}>{message.text}</Text>}
/>
```

## Update items

Give every item a stable, unique key. Replace arrays and changed items instead of mutating them. Ink reuses measurements for unchanged items and keeps the visible row in place when earlier rows load or change height.

Rows unmount when they leave the rendered window. Keep state that must survive scrolling in a parent component or store. All lists use the containing screen’s scroll position, not a separate scroll container.

Set `gap` to change the space between rows. Set `followEnd` to follow new items while the reader is near the bottom. Following pauses during a touch gesture or when the reader is more than 64 logical units from the end.

## Automatic pagination

Provide `onLoadMore: () => Promise<void>` and `hasMore`. Ink calls your function when the reader gets roughly one viewport from the last loaded row. Append the fetched items and set `hasMore` to false when there are no more.

Your app keeps the cursor and fetches data. Ink decides when to load and allows one request at a time. Normal loading shows no extra button or spinner; a failed request shows an error and retry action.

Short or empty lists can load enough content to fill the window. A successful request that adds no rows is not repeated at the same boundary. Use a new List key when changing the query or data source.

For older history, use `onLoadOlder` and `hasOlder`, then prepend the fetched items. Ink preserves the reader’s position. ConversationScreen already handles this pattern.

## Advanced composition

- `initialEnd` mounts the last rows first. It does not scroll the screen by itself. Use it for a custom history view whose screen starts at the bottom; ConversationScreen coordinates this for you.
- `measurementKey` clears cached offscreen measurements when layout changes without changing item data. For example, pass `measurementKey={textSize}` for an app-wide text-size setting. Normal item updates, width changes and image loads do not need it.

## Rendering and limits

Ink renders one viewport of extra rows before and after the visible area. It estimates unseen heights from measured rows. While new rows render, the screen keeps its last complete frame, so fast scrolling can briefly run ahead of the displayed content.

Measurements are cached by key, content version and width, including the width with a scrollbar. If the visible anchor is deleted, Ink uses the row at its old index or the last remaining row. With multiple lists, the anchor nearest the viewport top is used.

Item data stays in React. Ordered keys and content versions are limited to 128 KiB of UTF-8 JSON per list. Use compact keys and limit loaded history; oversized metadata reports an error.

Native height lookups and updates take logarithmic time. New dataset revisions and widths require linear preparation. Viewport requests keep only the newest pending destination, and stale revisions are ignored.

## Examples

The template includes a 5,000-row **Virtualised List**, a **Pagination** example with five 20-item pages, and **Follow new items** with pinned Add and Clear actions. These use local data; real-app paging still needs integration checks.
