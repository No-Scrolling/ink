import { createElement, useLayoutEffect, useMemo, useRef } from "react";
import { List } from "ink";
import { fixedListRow } from "ink/internal/list";
import { checkCircleFilled, videoFileFilled } from "ink/icons";

type MediaGridItem = { id: string; src: string; mimeType: string };
type MediaGridViewProps<T> = {
  items: readonly T[];
  selected?: ReadonlySet<string> | null;
  selectionStyle: "selected" | "dimmed";
  onPress: (item: T, index: number) => void;
  onLongPress?: (item: T, index: number) => void;
  hideIncompleteRow?: boolean;
  hasMore?: boolean;
  onLoadMore?: () => Promise<void>;
};

const mediaRow = /* @__PURE__ */ fixedListRow<ReturnType<typeof gridCell>[]>((node, field) => node("MediaGridRow", {},
  Array.from({ length: 3 }, (_, index) => node("MediaCell", {
    key: field(cells => cells[index].key),
    hidden: field(cells => cells[index].hidden), src: field(cells => cells[index].src),
    selected: field(cells => cells[index].selected), dimmed: field(cells => cells[index].dimmed),
    video: field(cells => cells[index].video), checkIcon: checkCircleFilled, videoIcon: videoFileFilled,
    onPress: field(cells => cells[index].onPress), onLongPress: field(cells => cells[index].onLongPress),
  }))));

type GridCell<T> = { item: T; index: number; selected: boolean; dimmed: boolean };
type GridRow<T> = { key: string; cells: GridCell<T>[]; longPress: boolean };
type GridHandlers<T> = Pick<MediaGridViewProps<T>, "onPress" | "onLongPress">;

function gridCell<T extends MediaGridItem>(handlers: { current: GridHandlers<T> }, row: GridRow<T>, index: number) {
  const entry = row.cells[index];
  return {
    key: entry?.item.id,
    hidden: !entry, src: (entry ?? row.cells[0]).item.src, selected: !!entry?.selected, dimmed: !!entry?.dimmed,
    video: !!entry?.item.mimeType.startsWith("video/"),
    // Handlers are read when pressed, so unchanged rows never hold stale callbacks.
    onPress: entry && (() => handlers.current.onPress(entry.item, entry.index)),
    onLongPress: entry && row.longPress ? () => handlers.current.onLongPress?.(entry.item, entry.index) : undefined,
  };
}

export function MediaGridView<T extends MediaGridItem>({ items, selected, selectionStyle, onPress, onLongPress, hideIncompleteRow, hasMore, onLoadMore }: MediaGridViewProps<T>) {
  const handlers = useRef<GridHandlers<T>>({ onPress, onLongPress });
  useLayoutEffect(() => { handlers.current = { onPress, onLongPress }; });
  const cache = useRef(new Map<string, GridRow<T>>());
  const longPress = !!onLongPress;
  const rows = useMemo(() => {
    const next = new Map<string, GridRow<T>>();
    const rows: GridRow<T>[] = [];
    const count = hideIncompleteRow && hasMore ? items.length - items.length % 3 : items.length;
    for (let index = 0; index < count; index += 3) {
      const cells = items.slice(index, index + 3).map((item, offset) => ({
        item, index: index + offset,
        selected: selectionStyle === "selected" && !!selected?.has(item.id),
        dimmed: selectionStyle === "dimmed" && selected != null && !selected.has(item.id),
      }));
      const key = cells[0].item.id;
      const previous = cache.current.get(key);
      // Rows keep their identity unless an item, its position or its presentation changes.
      const row = previous && previous.longPress === longPress && previous.cells.length === cells.length && previous.cells.every((cell, offset) =>
        cell.item === cells[offset].item && cell.index === cells[offset].index && cell.selected === cells[offset].selected && cell.dimmed === cells[offset].dimmed)
        ? previous : { key, cells, longPress };
      next.set(key, row);
      rows.push(row);
    }
    cache.current = next;
    return rows;
  }, [items, selected, selectionStyle, hideIncompleteRow, hasMore, longPress]);
  return createElement(List<GridRow<T>>, {
    items: rows, keyExtractor: row => row.key, gap: 0, hasMore, onLoadMore,
    renderItem: mediaRow((row: GridRow<T>) => Array.from({ length: 3 }, (_, index) => gridCell(handlers, row, index))),
  });
}
