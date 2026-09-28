export type NativeListItem = { key: string; values: unknown; measurementKey: string | number | null };
type Patch = { previous: NativeListItem[]; changes: NativeListItem[]; keys?: string[] };

const patches = new WeakMap<NativeListItem[], Patch>();

export function rememberListPatch(items: NativeListItem[], patch: Patch) {
  patches.set(items, patch);
}

export function listPatch(previous: NativeListItem[], items: NativeListItem[]) {
  const patch = patches.get(items);
  patches.delete(items);
  return patch?.previous === previous ? patch : undefined;
}
