import { createElement, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Button, EmptyState, ErrorState, List, LoadingState, Screen, Text } from "ink";
import { callNative, NativeError } from "ink/native";
import { files, type FileRef } from "./index";
import { decodeFileRef } from "./decode";
import icons from "./media.ink-icons";

type MediaItem = { id: string; src: string; name: string; mimeType: string; width: number; height: number };
type MediaPage = { items: MediaItem[]; nextCursor: string | null };
type Permission = "granted" | "denied" | "blocked";
export type MediaPickerProps = {
  kind?: "image" | "video" | "all";
  title?: string;
  onSelect: (files: FileRef[]) => void | Promise<void>;
};

function decodePage(value: string): MediaPage {
  const page: unknown = JSON.parse(value);
  if (typeof page !== "object" || page === null || !("items" in page) || !Array.isArray(page.items)
    || !("nextCursor" in page) || (page.nextCursor !== null && typeof page.nextCursor !== "string")) throw new NativeError("protocol", "Invalid media page");
  for (const item of page.items) {
    if (typeof item !== "object" || item === null || !["id", "src", "name", "mimeType"].every(key => typeof Reflect.get(item, key) === "string")
      || !["width", "height"].every(key => typeof Reflect.get(item, key) === "number")) throw new NativeError("protocol", "Invalid media item");
  }
  return page as MediaPage;
}

export function MediaPicker({ kind = "all", title = kind === "image" ? "Photos" : kind === "video" ? "Videos" : "Photos and videos", onSelect }: MediaPickerProps) {
  const [items, setItems] = useState<MediaItem[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [permission, setPermission] = useState<Permission | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [importing, setImporting] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const cursor = useRef<string | null>(null);
  const active = useRef<AbortController | null>(null);
  const loadingPage = useRef<AbortSignal | null>(null);
  const confirming = useRef<AbortSignal | null>(null);

  const loadPage = useCallback(async (signal: AbortSignal, first = false) => {
    if (loadingPage.current || signal.aborted) return;
    loadingPage.current = signal;
    try {
      const page = decodePage(await callNative("files", "media-list", { kind, cursor: first ? undefined : cursor.current, limit: 60 }, { signal }));
      if (signal.aborted) return;
      if (!first && page.nextCursor !== null && page.nextCursor === cursor.current) throw new NativeError("protocol", "Media pagination did not advance");
      setItems(previous => {
        const seen = new Set(first ? [] : previous.map(item => item.id));
        return [...(first ? [] : previous), ...page.items.filter(item => { if (seen.has(item.id)) return false; seen.add(item.id); return true; })];
      });
      cursor.current = page.nextCursor;
      setHasMore(page.nextCursor !== null);
    } finally { if (loadingPage.current === signal) loadingPage.current = null; }
  }, [kind]);

  const refresh = useCallback(async (request: boolean) => {
    active.current?.abort();
    const controller = new AbortController(); active.current = controller;
    loadingPage.current = null;
    setLoading(true); setError(null);
    try {
      const result = await callNative("files", "media-permission", { kind, request }, { signal: controller.signal, timeoutMs: 120_000 });
      if (!["granted", "denied", "blocked"].includes(result)) throw new NativeError("protocol", "Invalid photo access result");
      if (controller.signal.aborted) return;
      setPermission(result as Permission);
      if (result === "granted") await loadPage(controller.signal, true);
    } catch (reason) {
      if (!controller.signal.aborted) setError(reason instanceof Error ? reason : new Error(String(reason)));
    } finally { if (!controller.signal.aborted) setLoading(false); }
  }, [kind, loadPage]);

  useEffect(() => {
    setSelected(new Set()); setItems([]); cursor.current = null;
    confirming.current = null; setImporting(false);
    void refresh(true);
    return () => { active.current?.abort(); active.current = null; };
  }, [refresh]);

  const confirm = async () => {
    if (confirming.current || !selected.size || !active.current) return;
    const signal = active.current.signal;
    confirming.current = signal; setImporting(true); setError(null);
    const imported: FileRef[] = [];
    let delivered = false;
    try {
      for (const item of items.filter(item => selected.has(item.id))) {
        const file = decodeFileRef(await callNative("files", "media-import", { id: item.id }, { signal, timeoutMs: 600_000 }));
        if (!file) throw new NativeError("protocol", "Media import returned no file");
        imported.push(file);
      }
      signal.throwIfAborted();
      delivered = true;
      await onSelect(imported);
    } catch (reason) {
      if (!signal.aborted) setError(reason instanceof Error ? reason : new Error(String(reason)));
    } finally {
      if (!delivered) await Promise.all(imported.map(file => files.remove(file.id).catch(() => {})));
      if (confirming.current === signal) {
        confirming.current = null;
        if (!signal.aborted) setImporting(false);
      }
    }
  };

  const rows = useMemo(() => {
    const rows: MediaItem[][] = [];
    for (let index = 0; index < items.length; index += 3) rows.push(items.slice(index, index + 3));
    return rows;
  }, [items]);
  if (loading) return createElement(Screen, { title }, createElement(LoadingState, { label: "Loading library…" }));
  if (error) return createElement(Screen, { title }, createElement(ErrorState, {
    message: error.message,
    onRetry: () => { if (permission !== "granted" || items.length === 0) void refresh(false); else setError(null); },
  }));
  if (permission !== "granted") return createElement(Screen, { title },
    createElement(Text, { size: 18 }, `Allow access to all ${kind === "image" ? "photos" : kind === "video" ? "videos" : "photos and videos"} to browse the library.`),
    permission === "blocked" && createElement(Button, { onPress: () => { void callNative("files", "media-settings", {}).catch(reason => setError(reason instanceof Error ? reason : new Error(String(reason)))); } }, "Open settings"),
    createElement(Button, { onPress: () => { void refresh(permission !== "blocked"); } }, permission === "blocked" ? "Refresh access" : "Allow access"));
  if (importing) return createElement(Screen, { title }, createElement(LoadingState, { label: "Preparing attachments…" }));
  return createElement("MediaPickerScreen", { title, rightIcon: selected.size ? icons.check : undefined, onRightPress: selected.size ? confirm : undefined },
    items.length === 0 ? createElement(EmptyState, { title: kind === "video" ? "No videos" : kind === "image" ? "No photos" : "No photos or videos" }) :
      createElement(List<MediaItem[]>, {
        items: rows, keyExtractor: row => row[0].id, gap: 0, hasMore,
        onLoadMore: async () => { if (active.current) await loadPage(active.current.signal); },
        renderItem: row => createElement("MediaGridRow", null, row.map(item => createElement("MediaCell", {
          key: item.id, src: item.src, selected: selected.has(item.id), video: item.mimeType.startsWith("video/"), checkIcon: icons.check, videoIcon: icons.play_arrow,
          onPress: () => setSelected(previous => { const next = new Set(previous); if (next.has(item.id)) next.delete(item.id); else next.add(item.id); return next; }),
        }))),
      }));
}
