import { createStore, openDatabase } from "@ink/store";
import { resource } from "ink";

export type Item = { id: string; title: string; subtitle: string; titleSize: number };
export type Saved = { schema: 3; rows: readonly Item[] };
export type Boot = { mode?: "seed" | "persisted" | "local"; count?: number; value?: Saved };
export const boot = (Reflect.get(globalThis, "__inkJourneyBoot") ?? {}) as Boot;
declare const __inkPost: (message: string) => void;
export const post = (value: object) => __inkPost(JSON.stringify(value));
export const row = (id: string, subtitle = "Original"): Item => ({ id, title: `J ${id}`, subtitle, titleSize: 28 });
export const database = () => opened ??= openDatabase("asset://ink-assets/journeys.db");
let opened: ReturnType<typeof openDatabase> | undefined;
export async function closeDatabase() { if (opened) await (await opened).close(); }

export const saved = createStore<Saved>({
  key: "journey-library", version: 3, initial: { schema: 3, rows: [] },
  decode(value: unknown): Saved {
    if (typeof value !== "object" || value === null || !("schema" in value) || value.schema !== 3 ||
        !("rows" in value) || !Array.isArray(value.rows) || value.rows.some(item =>
          typeof item !== "object" || item === null || typeof item.id !== "string" ||
          typeof item.title !== "string" || typeof item.subtitle !== "string" || item.titleSize !== 28)) throw Error("Invalid saved library");
    return value as Saved;
  },
});

const attempts = new Map<string, number>();
export const routeResource = resource({
  key: (id: string) => ["journey-route", id], staleTime: 300_000,
  async load(id: string): Promise<readonly Item[]> {
    const attempt = (attempts.get(id) ?? 0) + 1;
    attempts.set(id, attempt);
    const db = await database();
    const result = await db.query(attempt === 1
      ? "SELECT id FROM deliberately_missing_route_table"
      : "SELECT id FROM catalogue WHERE kind = 'station' ORDER BY id DESC LIMIT 24");
    return result.map(item => row(String(item.id), `Route ${id}`));
  },
});
