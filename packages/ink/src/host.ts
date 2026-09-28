let nextId = 1;
const actions = new Map<number, Readonly<Record<string, (...args: unknown[]) => void>>>();
export type ViewData = null | boolean | number | string | readonly ViewData[] | { readonly [key: string]: ViewData };
export type ViewUpdate = { op: "values"; view: number; values: [number, ViewData][] };
const pendingValues = new Map<number, () => ViewUpdate | undefined>();
let valuesScheduled = false;
declare const __inkCommit: (operations: ViewUpdate[]) => void;

export function takeViewUpdates(): ViewUpdate[] {
  valuesScheduled = false;
  const pending = [...pendingValues.values()];
  pendingValues.clear();
  const operations: ViewUpdate[] = [];
  for (const update of pending) {
    const operation = update();
    if (operation) operations.push(operation);
  }
  return operations;
}

export function scheduleViewUpdate(id: number, update: () => ViewUpdate | undefined) {
  pendingValues.set(id, update);
  if (valuesScheduled) return;
  valuesScheduled = true;
  queueMicrotask(() => {
    const operations = takeViewUpdates();
    if (operations.length) __inkCommit(operations);
  });
}

export function cancelViewUpdate(id: number) { pendingValues.delete(id); }

export function allocateHostId() {
  if (!Number.isSafeInteger(nextId)) throw new RangeError("Native view identifiers exhausted");
  return nextId++;
}

export function registerHostActions(id: number, handlers: Readonly<Record<string, (...args: unknown[]) => void>>) {
  actions.set(id, handlers);
  return () => { actions.delete(id); };
}

export function dispatchHostAction(id: number, name: string, args: unknown[]) {
  actions.get(id)?.[name]?.(...args);
}
