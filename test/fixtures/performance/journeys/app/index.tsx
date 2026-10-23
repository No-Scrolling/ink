import { useEffect, useEffectEvent, useRef, useState } from "react";
import { Button, LoadingState, Screen, Stack, Text, TextInput, navigate, useSnapshot } from "ink";
import { onNativeMessage } from "ink/native";
import { boot, closeDatabase, database, post, row, saved, type Item } from "../data";
import { JourneyRows } from "../ui";
import { Local } from "../local";

function Persisted() {
  const snapshot = useSnapshot(saved);
  useEffect(() => {
    if (boot.mode === "seed") void saved.set(boot.value!).then(() => post({ type: "seeded" }));
  }, []);
  return <Screen title="Saved library" wide>
    {snapshot.status === "ready" ? <JourneyRows items={snapshot.data.rows} /> :
      snapshot.status === "error" ? <Text>{snapshot.error.message}</Text> : <LoadingState label="Opening saved library" />}
  </Screen>;
}

function Bench() {
  const [items, setItems] = useState<readonly Item[]>([row("r00000")]);
  const [draft, setDraft] = useState("");
  const [search, setSearch] = useState(false);
  const [arrivals, setArrivals] = useState<readonly Item[]>([]);
  const [home, setHome] = useState(0);
  const [route, setRoute] = useState("0");
  const [requested, setRequested] = useState(0);
  const statistics = useRef({ started: 0, aborted: 0, completed: 0, stale: 0, active: 0, ticket: 0, published: "" });

  useEffect(() => {
    if (!search) return;
    const controller = new AbortController();
    const stats = statistics.current;
    const ticket = ++stats.ticket;
    const token = `Search ${draft}:${arrivals.length}`;
    stats.active++;
    void (async () => {
      const db = await database();
      if (controller.signal.aborted) return;
      const pending = db.query(draft === "slow"
        ? "WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n) SELECT count(*) AS id FROM n"
        : "SELECT id FROM catalogue WHERE kind LIKE ? ORDER BY id", draft === "slow" ? [] : [`${draft}%`],
      { signal: controller.signal });
      stats.started++;
      post({ type: "query-started", draft, ticket });
      const result = await pending;
      stats.completed++;
      if (ticket !== stats.ticket || controller.signal.aborted) { stats.stale++; return; }
      const additions = "harbour".startsWith(draft) ? arrivals : [];
      setItems([...result.map(item => row(String(item.id), token)), ...additions.map(item => ({ ...item, subtitle: token }))]);
      stats.published = token;
    })().catch(error => {
      if (!controller.signal.aborted) throw error;
    }).finally(() => { stats.active--; });
    return () => { stats.aborted++; controller.abort(); };
  }, [search, draft, arrivals]);

  const receive = useEffectEvent((message: Record<string, unknown>) => {
    if (message.command === "populate") {
      setSearch(false);
      setItems(Array.from({ length: Number(message.count) }, (_, index) => row(`r${String(index).padStart(5, "0")}`)));
    }
    if (message.command === "insert") setItems(previous => [row(String(message.id)), ...previous]);
    if (message.command === "delete") setItems(previous => previous.filter(item => item.id !== message.id));
    if (message.command === "reorder") setItems(previous => [...previous].reverse());
    if (message.command === "resize") setItems(previous => previous.map((item, index) => index === 0 ? {
      ...item, titleSize: message.expanded ? 40 : 28,
      title: message.expanded ? `J ${item.id} · An evening recording from the old theatre beside the river in Zürich, with the harbour ensemble and their guests performing the complete concert` : `J ${item.id}`,
    } : item));
    if (message.command === "search") { setArrivals([]); setDraft("slow"); setSearch(true); }
    if (message.command === "arrival") setArrivals(previous => [...previous, row(String(message.id))]);
    if (message.command === "route") setRoute(String(message.id));
    if (typeof message.sequence === "number") setRequested(message.sequence);
    if (message.command === "report") post({ type: "observed", draft, received: arrivals.length,
      ids: items.map(item => item.id), home, statistics: statistics.current });
    if (message.command === "close") void closeDatabase().then(() => post({ type: "closed" }));
  });
  useEffect(() => {
    const stop = onNativeMessage("benchmark", receive);
    post({ type: "ready" });
    return stop;
  }, []);
  return <Screen title="Journeys" wide>
    <Stack axis="horizontal" gap={8}>
      <Button onPress={() => setHome(value => value + 1)}>Save home</Button>
      <Button onPress={() => navigate({ path: "/detail/[id]", params: { id: route } })}>Open route</Button>
    </Stack>
    <Text width={800} maxLines={1}>Home {home}</Text>
    <TextInput value={draft} onChange={setDraft} autoFocus placeholder="Search catalogue" action="search" />
    <Text width={800} maxLines={1}>{`Requested ${requested}`}</Text>
    <JourneyRows items={items} />
  </Screen>;
}

export default function App() { return boot.mode === "local" ? <Local /> : boot.mode ? <Persisted /> : <Bench />; }
