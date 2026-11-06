import { createElement, useMemo, useRef, type ReactNode } from "react";
import { actionBarProps, List, type ActionBarItem, type IconAsset } from "ink";
import { nativeListRow, nativeListTemplate } from "ink/internal/list";
import { callNative, NativeError, onNativeMessage } from "ink/native";
import { checkCircleFilled, videoFileFilled } from "ink/icons";

export type MediaKind = "image" | "video" | "all";

export type MediaItem = {
  id: string;
  src: string;
  name: string;
  mimeType: string;
  width: number;
  height: number;
  /** Seconds since the epoch when the item was added to the library. */
  added: number;
  duration?: number;
};

export type MediaPage = { items: MediaItem[]; nextCursor: string | null; version: string };
export type MediaCounts = { all: number; image: number; video: number; groups: Record<string, number> };

function decodeItem(item: unknown): MediaItem {
  if (typeof item !== "object" || item === null || !["id", "src", "name", "mimeType"].every(key => typeof Reflect.get(item, key) === "string")
    || !["width", "height", "added"].every(key => typeof Reflect.get(item, key) === "number")) throw new NativeError("protocol", "Invalid media item");
  return item as MediaItem;
}

export const mediaLibrary = {
  /** Newest additions first. Pass the previous page's `nextCursor` to continue. */
  async list(options: { kind?: MediaKind; ids?: readonly string[]; cursor?: string | null; limit?: number; signal?: AbortSignal } = {}): Promise<MediaPage> {
    const page: unknown = JSON.parse(await callNative("files", "media-list", {
      kind: options.kind ?? "all", ids: options.ids, cursor: options.cursor ?? undefined, limit: options.limit ?? 60, order: "added", previews: true,
    }, { signal: options.signal }));
    if (typeof page !== "object" || page === null || !("items" in page) || !Array.isArray(page.items) || !("nextCursor" in page) || !("version" in page)
      || (page.nextCursor !== null && typeof page.nextCursor !== "string") || typeof page.version !== "string") throw new NativeError("protocol", "Invalid media page");
    return { items: page.items.map(decodeItem), nextCursor: page.nextCursor, version: page.version };
  },
  /** Counts the whole library and optional groups of IDs without loading their media items. */
  async counts(options: { groups?: Readonly<Record<string, readonly string[]>>; signal?: AbortSignal } = {}): Promise<MediaCounts> {
    const counts: unknown = JSON.parse(await callNative("files", "media-counts", { groups: options.groups ?? {} }, { signal: options.signal }));
    const count = (value: unknown): value is number => typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
    if (typeof counts !== "object" || counts === null || !("all" in counts) || !count(counts.all)
      || !("image" in counts) || !count(counts.image) || !("video" in counts) || !count(counts.video)
      || !("groups" in counts) || typeof counts.groups !== "object" || counts.groups === null || Array.isArray(counts.groups)
      || !Object.values(counts.groups).every(count)) throw new NativeError("protocol", "Invalid media counts");
    return counts as MediaCounts;
  },
  /** Deletes items from the device. Resolves with the deleted IDs, or null when declined. */
  async remove(ids: readonly string[]): Promise<string[] | null> {
    const result: unknown = JSON.parse(await callNative("files", "media-delete", { ids }, { timeoutMs: 600_000 }));
    if (result === null) return null;
    if (!Array.isArray(result) || !result.every(id => typeof id === "string")) throw new NativeError("protocol", "Invalid deleted media IDs");
    return result;
  },
  /** Rotates a photo clockwise. Resolves with the updated item, or null when declined. */
  async rotate(id: string, degrees: 90 | 180 | 270): Promise<MediaItem | null> {
    const result: unknown = JSON.parse(await callNative("files", "media-rotate", { id, degrees }, { timeoutMs: 600_000 }));
    return result === null ? null : decodeItem(result);
  },
  /** Prepares instant grid previews for the supplied library items in the background. */
  async preparePreviews(options: { ids: readonly string[]; signal?: AbortSignal }): Promise<number> {
    return Number(await callNative("files", "media-previews", { ids: options.ids }, { signal: options.signal }));
  },
  /** Calls `listener` after photos or videos are added, changed or removed, including while the app was hidden. */
  watch(listener: (version: string) => void, onError: (error: Error) => void): () => void {
    watchers.set(listener, onError);
    const request = watchRequest ??= callNative("files", "media-watch", {});
    void request.catch(error => {
      if (watchRequest === request) watchRequest = null;
      if (watchers.get(listener) === onError) onError(error instanceof Error ? error : new Error(String(error)));
    });
    return () => {
      watchers.delete(listener);
      if (!watchers.size) {
        watchRequest = null;
        void callNative("files", "media-unwatch", {}).catch(error => onError(error instanceof Error ? error : new Error(String(error))));
      }
    };
  },
};

const watchers = new Map<(version: string) => void, (error: Error) => void>();
let watchRequest: Promise<string> | null = null;
onNativeMessage("media-changed", message => {
  if (typeof message.data !== "object" || message.data === null || !("version" in message.data) || typeof message.data.version !== "string") {
    throw new NativeError("protocol", "Invalid media library change");
  }
  for (const listener of [...watchers.keys()]) listener(message.data.version);
});

/** Full-resolution source for a photo, decoded at the size it is displayed. */
export function fullSource(item: MediaItem) {
  return item.src + "&full=1";
}

/** A `content://` URI for video playback. */
export function contentUri(item: MediaItem) {
  const [kind, id] = item.id.split(":");
  return `content://media/external/${kind === "video" ? "video" : "images"}/media/${id}`;
}

const gridTemplate = nativeListTemplate((node, field) => node("MediaGridRow", {},
  Array.from({ length: 3 }, (_, index) => node("MediaCell", {
    hidden: field(index, "hidden"), src: field(index, "src"), dimmed: field(index, "dimmed"),
    video: field(index, "video"), checkIcon: checkCircleFilled, videoIcon: videoFileFilled,
    onPress: field(index, "onPress"), onLongPress: field(index, "onLongPress"),
  }))));

export type MediaGridProps = {
  items: readonly MediaItem[];
  /** A selection dims the other items. Omit it or pass null to browse normally. */
  selected?: ReadonlySet<string> | null;
  onPress: (item: MediaItem, index: number) => void;
  onLongPress?: (item: MediaItem, index: number) => void;
  hasMore?: boolean;
  onLoadMore?: () => Promise<void>;
};

type GridCell = { item: MediaItem; index: number; dimmed: boolean };
type GridRow = { key: string; cells: GridCell[] };
type GridHandlers = Pick<MediaGridProps, "onPress" | "onLongPress">;

function gridCell(handlers: { current: GridHandlers }, entry: GridCell | undefined, first: GridCell) {
  return {
    hidden: !entry, src: (entry ?? first).item.src, dimmed: !!entry?.dimmed,
    video: !!entry?.item.mimeType.startsWith("video/"),
    // Handlers are read when pressed, so unchanged rows never hold stale callbacks.
    onPress: entry && (() => handlers.current.onPress(entry.item, entry.index)),
    onLongPress: entry && (() => handlers.current.onLongPress?.(entry.item, entry.index)),
  };
}

/** Three-column, edge-to-edge grid of library items. Place it in `MediaGridScreen`. */
export function MediaGrid({ items, selected, onPress, onLongPress, hasMore, onLoadMore }: MediaGridProps) {
  const handlers = useRef<GridHandlers>({ onPress, onLongPress });
  handlers.current = { onPress, onLongPress };
  const cache = useRef(new Map<string, GridRow>());
  const rows = useMemo(() => {
    const next = new Map<string, GridRow>();
    const rows: GridRow[] = [];
    const count = hasMore ? items.length - items.length % 3 : items.length;
    for (let index = 0; index < count; index += 3) {
      const cells = items.slice(index, index + 3).map((item, offset) => ({
        item, index: index + offset, dimmed: selected != null && !selected.has(item.id),
      }));
      const key = cells[0].item.id;
      const previous = cache.current.get(key);
      // Rows keep their identity unless an item or its selection changes, so only those rows update.
      const row = previous && previous.cells.length === cells.length && previous.cells.every((cell, offset) =>
        cell.item === cells[offset].item && cell.index === cells[offset].index && cell.dimmed === cells[offset].dimmed) ? previous : { key, cells };
      next.set(key, row);
      rows.push(row);
    }
    cache.current = next;
    return rows;
  }, [items, selected, hasMore]);
  return createElement(List<GridRow>, {
    items: rows, keyExtractor: row => row.key, gap: 0, hasMore, onLoadMore,
    renderItem: nativeListRow((row: GridRow) => createElement("MediaGridRow", null, row.cells.map(entry => createElement("MediaCell", {
      key: entry.item.id, ...gridCell(handlers, entry, row.cells[0]), checkIcon: checkCircleFilled, videoIcon: videoFileFilled,
    }))), row => [Array.from({ length: 3 }, (_, index) => gridCell(handlers, row.cells[index], row.cells[0]))], gridTemplate),
  });
}

/** A screen without margins for `MediaGrid`, with an optional bottom action bar. */
export function MediaGridScreen({ title, onTitlePress, titleIcon, rightAction, actionBar, children }: {
  title?: string;
  onTitlePress?: () => void;
  titleIcon?: IconAsset;
  rightAction?: { icon: IconAsset; onPress: () => void };
  actionBar?: readonly ActionBarItem[];
  children?: ReactNode;
}) {
  return createElement("MediaPickerScreen", {
    title, onTitlePress, titleIcon, scrollBar: "auto", rightIcon: rightAction?.icon, onRightPress: rightAction?.onPress, ...actionBarProps(actionBar),
  }, children);
}
