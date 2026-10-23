import { useEffect, useState } from "react";
import * as Ink from "ink";
import { Button, List, Row, Screen, Stack, Text, useSnapshot } from "ink";
import { onNativeMessage } from "ink/native";
import type { Source, SourceSnapshot, SourceView } from "ink";

declare const __inkPost: (message: string) => void;
type Item = { id: string; title: string; subtitle: string; read: boolean };
const native = typeof Reflect.get(globalThis, "__inkSource") === "function";
const createItems = (count: number, generation = 0): Item[] => Array.from({ length: count }, (_, index) => ({
  id: String(index), title: `Message ${String(index).padStart(5, "0")}`,
  subtitle: generation ? `Refresh ${generation}` : "Original", read: index % 2 === 1,
}));
const post = (value: object) => __inkPost(JSON.stringify(value));

function Count({ view }: { view: SourceView<Item> }) {
  const snapshot = useSnapshot(view);
  return <Text width={700} maxLines={1}>Unread: {snapshot.count}</Text>;
}

function Controls({ edit, replace }: { edit: () => void; replace: () => void }) {
  return <Stack axis="horizontal" gap={16}>
    <Button onPress={edit}>Edit</Button><Button onPress={replace}>Replace</Button>
  </Stack>;
}

function Native() {
  const [source] = useState(() => Ink.createSource(createItems(1000), { key: "id" }));
  const [unread] = useState(() => source.view({ read: false }));
  const [selected, setSelected] = useState("none");
  let retained: SourceSnapshot<Item> | undefined;
  const [commands] = useState(() => {
    let sequence = 0, count = 1000;
    return {
      edit() {
        const value = ++sequence;
        const current = source.getSnapshot().get("0")!;
        source.set({ ...current, subtitle: `Edit ${value}`, read: value % 2 === 1 });
      },
      replace() {
        source.replace(createItems(count, ++sequence));
      },
      populate(size: number) { count = size; sequence = 0; source.replace(createItems(size)); },
    };
  });
  useEffect(() => {
    retained = source.getSnapshot();
    const stop = onNativeMessage("benchmark", message => {
      if (message.command === "populate") {
        commands.populate(Number(message.count));
        retained = source.getSnapshot();
      }
      if (message.command === "edit") commands.edit();
      if (message.command === "replace") commands.replace();
      if (message.command === "reorder") {
        const snapshot = source.getSnapshot();
        source.replace(Array.from({ length: snapshot.count }, (_, i) => snapshot.at(snapshot.count - i - 1)!));
      }
      if (message.command === "contracts") checkContracts(source, unread, retained!);
    });
    post({ type: "ready", native });
    return () => { stop(); source.dispose(); };
  }, [source, unread, commands]);
  return <Screen title={`Inbox ${selected}`} wide>
    <Controls edit={commands.edit} replace={commands.replace} />
    <Count view={unread} />
    <Ink.SourceList view={source} title="title" subtitle="subtitle" titleMaxLines={2} gap={8}
      onPress={key => setSelected(source.getSnapshot().get(key)!.subtitle)} />
  </Screen>;
}

function checkContracts(source: Source<Item>, unread: SourceView<Item>, retained: SourceSnapshot<Item>) {
  const current = source.getSnapshot();
  const fail = (message: string): never => { throw Error(message); };
  if (retained.get("0")?.subtitle !== "Original") fail("Retained snapshot changed");
  if (!Object.isFrozen(current.get("0"))) fail("Snapshot records must be frozen");
  const before = current.revision;
  for (const update of [
    () => source.transact([{ ...current.get("0")!, subtitle: "invalid" }], before - 1),
    () => source.transact([current.get("0")!, current.get("0")!]),
    () => source.set({ id: "unknown", title: "Unknown", subtitle: "invalid", read: false }),
  ]) {
    let rejected = false;
    try { update(); } catch { rejected = true; }
    if (!rejected || source.getSnapshot().revision !== before) fail("Invalid transaction was not atomic");
  }
  let count = 0;
  for (let index = 0; index < current.count; index++) if (!current.at(index)!.read) count++;
  if (unread.getSnapshot().count !== count) fail("Unread view count is wrong");
  const temporary = source.view({ read: false });
  const old = temporary.getSnapshot();
  temporary.dispose();
  if (old.count !== count || (old.count && old.at(0)?.read !== false)) fail("Released snapshot is unreadable");
  const probe = Ink.createSource<{ id: string; group: string; flag?: boolean | null }>([
    { id: "a", group: "one", flag: false }, { id: "b", group: "two", flag: null }, { id: "c", group: "one" },
  ], { key: "id" });
  const filtered = probe.view({ group: "one", flag: false });
  const nulls = probe.view({ flag: null });
  const saved = filtered.getSnapshot();
  let notifications = 0;
  filtered.subscribe(() => { notifications++; });
  probe.transact([{ id: "a", group: "one", flag: false }]);
  if (notifications !== 0) fail("Identical replacements notified subscribers");
  probe.transact([{ id: "a", group: "two", flag: false }, { id: "c", group: "one", flag: false }]);
  if (filtered.getSnapshot().count !== 1 || filtered.getSnapshot().at(0)?.id !== "c" || notifications !== 1) fail("Filtered membership crossing failed");
  if (nulls.getSnapshot().count !== 1 || nulls.getSnapshot().at(0)?.id !== "b") fail("Missing fields matched null");
  probe.replace([{ id: "c", group: "one", flag: false }, { id: "a", group: "one", flag: false }]);
  if (filtered.getSnapshot().at(0)?.id !== "c" || filtered.getSnapshot().at(1)?.id !== "a") fail("Filtered replacement order failed");
  probe.dispose();
  if (saved.at(0)?.id !== "a" || saved.count !== 1) fail("Retained filtered snapshot changed after release");
  post({ type: "contracts", passed: true, retained: true, atomic: true, count });
}

function Arrays() {
  const [items, setItems] = useState(() => createItems(1000));
  const [selected, setSelected] = useState("none");
  const [commands] = useState(() => {
    let sequence = 0, count = 1000;
    return {
      edit() {
        const value = ++sequence;
        setItems(previous => {
          const next = previous.slice();
          next[0] = { ...next[0], subtitle: `Edit ${value}`, read: value % 2 === 1 };
          return next;
        });
      },
      replace() {
        setItems(createItems(count, ++sequence));
      },
      populate(size: number) { count = size; sequence = 0; setItems(createItems(size)); },
    };
  });
  useEffect(() => {
    const stop = onNativeMessage("benchmark", message => {
      if (message.command === "populate") commands.populate(Number(message.count));
      if (message.command === "edit") commands.edit();
      if (message.command === "replace") commands.replace();
      if (message.command === "reorder") setItems(previous => [...previous].reverse());
      if (message.command === "contracts") post({ type: "contracts", passed: true, native: false });
    });
    post({ type: "ready", native });
    return stop;
  }, [commands]);
  return <Screen title={`Inbox ${selected}`} wide>
    <Controls edit={commands.edit} replace={commands.replace} />
    <Text width={700} maxLines={1}>Unread: {items.filter(item => !item.read).length}</Text>
    <List items={items} gap={8} keyExtractor={item => item.id}
      renderItem={item => <Row title={item.title} titleMaxLines={2} subtitle={item.subtitle}
        onPress={() => setSelected(item.subtitle)} />} />
  </Screen>;
}

export default function App() { return native ? <Native /> : <Arrays />; }
