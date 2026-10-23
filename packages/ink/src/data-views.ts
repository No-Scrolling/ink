type IntrinsicCalls = {
  filter<T>(items: readonly T[], predicate: (item: T, index: number) => boolean): T[];
  sort<T>(items: T[], compare: (a: T, b: T) => number): T[];
  map<T, U>(items: readonly T[], transform: (item: T, index: number) => U): U[];
  lower(text: string): string;
  includes(text: string, query: string): boolean;
  localeCompare(a: string, b: string): number;
  dateParse(value: unknown): number;
  jsonParse(source: string, reviver: (this: object, key: string, value: unknown) => unknown): unknown;
  weakMapGet<T>(map: WeakMap<object, T>, key: object): T | undefined;
  weakMapSet<T>(map: WeakMap<object, T>, key: object, value: T): unknown;
  mapGet<K, V>(map: Map<K, V>, key: K): V | undefined;
  mapSet<K, V>(map: Map<K, V>, key: K, value: V): unknown;
};

type Intrinsics = {
  WeakMap: WeakMapConstructor;
  Map: MapConstructor;
  calls?: IntrinsicCalls;
  setTimeout?: (callback: () => void, delay: number) => number;
  clearTimeout?: (timer: number) => void;
};

declare const __inkDataIntrinsics: Intrinsics | undefined;
declare const __inkDataFields: ((items: unknown, fields: readonly string[], previous: readonly unknown[] | undefined,
  mode: "search" | "sort") => readonly unknown[] | undefined) | undefined;
declare const __inkDataSearch: ((items: unknown, fields: readonly string[], previousSnapshot: readonly unknown[] | undefined,
  previousText: readonly string[] | undefined, separator: string | null) => SearchCache | undefined) | undefined;
declare const __inkPrepareDataSearch: ((items: unknown, fields: readonly string[], separator: string | null,
  cursor: SearchCursor | undefined, limit: number) => SearchCursor | null | undefined) | undefined;

const intrinsics = typeof __inkDataIntrinsics === "undefined" ? undefined : __inkDataIntrinsics;
const calls = intrinsics?.calls;
const snapshotFields = typeof __inkDataFields === "function" ? __inkDataFields : undefined;
const prepareSearch = typeof __inkDataSearch === "function" ? __inkDataSearch : undefined;
const warmSearch = typeof __inkPrepareDataSearch === "function" ? __inkPrepareDataSearch : undefined;
const schedule = intrinsics?.setTimeout;
const unschedule = intrinsics?.clearTimeout;

type RawPlan = { fields: unknown; separator: unknown; queryLower: unknown; date: unknown; direction: unknown; tie: unknown };
type SearchCache = { snapshot: readonly unknown[]; text: readonly string[] };
interface CachedSearch extends SearchCache { changes: number }
type SearchCursor = SearchCache & { offset: number; complete: boolean };
type SearchJob = { items: object | undefined; cursor: SearchCursor | undefined; timer: number | undefined };
type SearchPlan = {
  fields: readonly string[];
  separator: string | null;
  queryLower: boolean;
  cache: WeakMap<object, CachedSearch | undefined>;
  seen: WeakMap<object, boolean>;
  job: SearchJob | undefined;
  retained: boolean;
  next: SearchPlan | undefined;
};
type SortPlan = { fields: readonly string[]; direction: 1 | -1; tie: boolean };
type SortEntry<T> = { item: T; date: number; tie: string };

// Plans are compiler literals. Bound retained plans as well as keeping source arrays weak.
const maximumPlans = 64;
let searches: Map<string, SearchPlan> | undefined;
let searchCount = 0;
let searchHead: SearchPlan | undefined;
let sorts: Map<string, SortPlan> | undefined;
let sortCount = 0;

function parsePlan(source: string): RawPlan | undefined {
  const i = intrinsics!;
  const records = new i.WeakMap<object, RawPlan>();
  try {
    // Collect only JSON's own members, including when Object.prototype has acquired getters.
    const parsed = calls!.jsonParse(source, function(this: object, key: string, value: unknown) {
      if (key === "fields" || key === "separator" || key === "queryLower" || key === "date" || key === "direction" || key === "tie") {
        let record = calls!.weakMapGet(records, this);
        if (!record) {
          record = { fields: undefined, separator: undefined, queryLower: undefined, date: undefined, direction: undefined, tie: undefined };
          calls!.weakMapSet(records, this, record);
        }
        record[key] = value;
      }
      return value;
    });
    return parsed !== null && typeof parsed === "object"
      ? calls!.weakMapGet(records, parsed) : undefined;
  } catch {
    return undefined;
  }
}

function searchPlan(source: string): SearchPlan | undefined {
  const i = intrinsics!;
  const known = searches && calls!.mapGet(searches, source);
  if (known) return known;
  const raw = parsePlan(source);
  if (!raw || raw.separator !== null && typeof raw.separator !== "string"
    || raw.queryLower !== undefined && typeof raw.queryLower !== "boolean") return undefined;
  // The native guard validates the fields array without invoking getters or Proxy traps.
  const plan: SearchPlan = { fields: raw.fields as readonly string[], separator: raw.separator,
    queryLower: raw.queryLower === true, cache: new i.WeakMap<object, CachedSearch | undefined>(),
    seen: new i.WeakMap<object, boolean>(), job: undefined, retained: true, next: undefined };
  if (!searches || searchCount === maximumPlans) {
    while (searchHead) {
      const evicted = searchHead;
      searchHead = evicted.next;
      evicted.next = undefined;
      evicted.retained = false;
      cancelSearch(evicted);
    }
    searches = new i.Map<string, SearchPlan>();
    searchCount = 0;
  }
  calls!.mapSet(searches, source, plan);
  plan.next = searchHead;
  searchHead = plan;
  searchCount++;
  return plan;
}

function cancelSearch(plan: SearchPlan): void {
  const job = plan.job;
  if (!job) return;
  plan.job = undefined;
  const timer = job.timer;
  job.items = undefined;
  job.cursor = undefined;
  job.timer = undefined;
  if (timer !== undefined) {
    try { unschedule!(timer); } catch { /* Cancellation has already invalidated the job. */ }
  }
}

function disableSearch(plan: SearchPlan, items: object): void {
  calls!.weakMapSet(plan.cache, items, undefined);
  calls!.weakMapSet(plan.seen, items, false);
}

function warmLater(plan: SearchPlan, items: object): void {
  if (!plan.retained || plan.job?.items === items) return;
  cancelSearch(plan);
  const job: SearchJob = { items, cursor: undefined, timer: undefined };
  plan.job = job;
  const step = () => {
    if (plan.job !== job) return;
    job.timer = undefined;
    try {
      const source = job.items!;
      const cursor = warmSearch!(source, plan.fields, plan.separator, job.cursor, 128);
      if (plan.job !== job) return;
      if (!cursor) {
        // Unsupported shapes stay on the original path. A shrinking source (null)
        // and thrown background errors remain eligible for a later attempt.
        if (cursor === undefined) calls!.weakMapSet(plan.seen, source, false);
        cancelSearch(plan);
      } else if (cursor.complete) {
        calls!.weakMapSet(plan.cache, source, { snapshot: cursor.snapshot, text: cursor.text, changes: 0 });
        cancelSearch(plan);
      } else {
        job.cursor = cursor;
        job.timer = schedule!(step, 0);
      }
    } catch {
      // Background work must never turn a successful original search into an error.
      if (plan.job === job) cancelSearch(plan);
    }
  };
  try { job.timer = schedule!(step, 0); } catch { cancelSearch(plan); }
}

function sortPlan(source: string): SortPlan | undefined {
  const i = intrinsics!;
  const known = sorts && calls!.mapGet(sorts, source);
  if (known) return known;
  const raw = parsePlan(source);
  if (!raw || typeof raw.date !== "string" || raw.direction !== 1 && raw.direction !== -1
    || raw.tie !== undefined && typeof raw.tie !== "string") return undefined;
  const plan: SortPlan = { fields: typeof raw.tie === "string" ? [raw.date, raw.tie] : [raw.date],
    direction: raw.direction, tie: typeof raw.tie === "string" };
  if (!sorts || sortCount === maximumPlans) {
    sorts = new i.Map<string, SortPlan>();
    sortCount = 0;
  }
  calls!.mapSet(sorts, source, plan);
  sortCount++;
  return plan;
}

/** @internal The compiler supplies the original filter expression as the fallback. */
export function compiledSearch<T>(items: readonly T[], source: string, query: () => unknown, fallback: (items: readonly T[]) => T[]): T[] {
  if (!intrinsics || !calls || !prepareSearch || !warmSearch || !schedule || !unschedule) return fallback(items);
  let plan: SearchPlan | undefined;
  try { plan = searchPlan(source); } catch { return fallback(items); }
  if (!plan) return fallback(items);
  if (plan.job && plan.job.items !== items) cancelSearch(plan);
  const before = calls.weakMapGet(plan.cache, items);
  if (!before) {
    const result = fallback(items);
    if (items !== null && (typeof items === "object" || typeof items === "function")) {
      try {
        const seen = calls.weakMapGet(plan.seen, items);
        if (seen === true) warmLater(plan, items);
        else if (seen === undefined) calls.weakMapSet(plan.seen, items, true);
      } catch { /* Retaining a warming candidate is optional. */ }
    }
    return result;
  }
  const prepared = prepareSearch(items, plan.fields, before.snapshot, before.text, plan.separator);
  if (!prepared) {
    disableSearch(plan, items);
    return fallback(items);
  }
  if (items.length === 0) return fallback(items);
  // A rejected guard or empty source must not evaluate a query (which may still be in its TDZ).
  const value = query();
  if (typeof value !== "string") return fallback(items);
  const needle = plan.queryLower ? calls.lower(value) : value;
  const result = calls.filter(items, (_item: T, index: number) => calls.includes(prepared.text[index], needle));
  const changes = prepared.snapshot === before.snapshot ? 0 : before.changes + 1;
  if (changes >= 2) disableSearch(plan, items);
  else calls.weakMapSet(plan.cache, items, { snapshot: prepared.snapshot, text: prepared.text, changes });
  return result;
}

/** @internal Sort date keys once per invocation while retaining Array.sort's source identity. */
export function compiledSort<T>(items: T[], source: string, fallback: (items: T[]) => T[]): T[] {
  if (!intrinsics || !calls || !snapshotFields) return fallback(items);
  const plan = sortPlan(source);
  if (!plan) return fallback(items);
  const snapshot = snapshotFields(items, plan.fields, undefined, "sort");
  if (!snapshot || items.length < 2) return fallback(items);
  const width = plan.fields.length;
  for (let offset = 0; offset < snapshot.length; offset += width) {
    const date = snapshot[offset];
    if (date !== null && typeof date === "object" || typeof date === "function" || typeof date === "symbol"
      || plan.tie && typeof snapshot[offset + 1] !== "string") return fallback(items);
  }
  const entries = calls.map(items, (item: T, index: number): SortEntry<T> => ({ item,
    date: calls.dateParse(snapshot[index * width] ?? "") || 0,
    tie: plan.tie ? snapshot[index * width + 1] as string : "" }));
  calls.sort(entries, (a: SortEntry<T>, b: SortEntry<T>) => {
    const order = plan.direction * (a.date - b.date);
    return order || (plan.tie ? calls.localeCompare(a.tie, b.tie) : 0);
  });
  for (let index = 0; index < entries.length; index++) items[index] = entries[index].item;
  return items;
}
