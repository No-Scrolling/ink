import { useSyncExternalStore } from "react";

export type Snapshot<T> =
  | { status: "loading" }
  | { status: "ready"; data: T }
  | { status: "error"; error: Error };

export interface SnapshotSource<T> {
  subscribe(listener: () => void): () => void;
  getSnapshot(): Snapshot<T>;
}

export function useSnapshot<T>(source: SnapshotSource<T>): Snapshot<T> {
  return useSyncExternalStore(source.subscribe, source.getSnapshot, source.getSnapshot);
}
