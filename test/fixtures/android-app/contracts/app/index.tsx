import "@ink/network";
import { Screen, Text } from "ink";
import { useEffect } from "react";
import { createStore } from "@ink/store";

const origin = "http://127.0.0.1:18765";
const decode = (value: unknown) => {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) throw new Error("Expected a stored integer");
  return value;
};
const message = (error: unknown) => error instanceof Error ? error.message : String(error);

async function observe() {
  const phase = Number(await (await fetch(`${origin}/phase`)).text());
  const details: Record<string, unknown> = {};
  try {
    let migrations = 0;
    const current = createStore({ key: "counter", version: 2, initial: 0, decode, migrate: value => {
      migrations++;
      return decode(value) + 1;
    } });
    if (phase === 2) {
      details.persisted = await current.get();
      details.migrations = migrations;
    } else {
      const old = createStore({ key: "counter", version: 1, initial: 0, decode });
      await old.set(41);
      details.migrated = await current.get();
      details.migrations = migrations;
      const sibling = createStore({ key: "counter", version: 2, initial: 0, decode });
      const watcher = createStore({ key: "counter", version: 2, initial: 0, decode });
      let unsubscribe = () => {};
      let timer: ReturnType<typeof setTimeout> | undefined;
      const changed = new Promise<number>((resolve, reject) => {
        let updates: Promise<void[]> | undefined;
        timer = setTimeout(() => reject(new Error("Store subscription did not receive the committed value")), 20_000);
        unsubscribe = watcher.subscribe(() => {
          const snapshot = watcher.getSnapshot();
          if (snapshot.status === "error") { reject(snapshot.error); return; }
          if (snapshot.status !== "ready") return;
          // Complete the passive watcher's initial load before either writer changes the store.
          if (snapshot.data === 42 && !updates) {
            details.committedMigration = snapshot.data;
            updates = Promise.all([current.update(value => value + 1), sibling.update(value => value + 1)]);
            void updates.catch(reject);
          }
          if (snapshot.data === 44 && updates) void updates.then(() => resolve(snapshot.data), reject);
        });
      });
      try {
        details.subscribed = await changed;
        details.concurrent = await current.get();
      } finally {
        unsubscribe();
        clearTimeout(timer);
      }
      try { await old.get(); details.downgrade = "accepted"; }
      catch (error) { details.downgrade = message(error); }

      const upload = new Uint8Array([0, 255, 99, 97, 102, 195, 169]);
      const echo = await fetch(`${origin}/echo`, {
        method: "POST", headers: { "x-ink-fixture": "binary" }, body: upload,
      });
      details.echo = { status: echo.status, header: echo.headers.get("x-ink-reply"), bytes: Array.from(await echo.bytes()) };
      const retained = await fetch(`${origin}/redirect307`, { method: "POST", body: upload });
      details.redirect307 = { redirected: retained.redirected, url: retained.url, bytes: Array.from(await retained.bytes()) };
      const rewritten = await fetch(`${origin}/redirect303`, { method: "POST", body: "discard me" });
      details.redirect303 = { redirected: rewritten.redirected, body: await rewritten.text() };

      const streamed = await fetch(`${origin}/stream`);
      const reader = streamed.body!.getReader();
      const bytes: number[] = [];
      try {
        for (;;) {
          const chunk = await reader.read();
          if (chunk.done) break;
          bytes.push(...chunk.value);
        }
      } finally { reader.releaseLock(); }
      details.stream = { bytes, bodyUsed: streamed.bodyUsed };

      const controller = new AbortController();
      const held = await fetch(`${origin}/held`, { signal: controller.signal });
      const heldReader = held.body!.getReader();
      try {
        details.beforeAbort = Array.from((await heldReader.read()).value ?? []);
        controller.abort("fixture-abort");
        try { await heldReader.read(); details.abort = "accepted"; }
        catch (error) { details.abort = message(error); }
      } finally { heldReader.releaseLock(); }
      const preAborted = new AbortController();
      preAborted.abort("before-send");
      try { await fetch(`${origin}/must-not-arrive`, { signal: preAborted.signal }); details.preAbort = "accepted"; }
      catch (error) { details.preAbort = message(error); }
      details.recovered = await (await fetch(`${origin}/healthy`)).text();
    }
    console.log("INK_CONTRACT_RESULT " + JSON.stringify({ version: 1, phase, details }));
  } catch (error) {
    console.error("INK_CONTRACT_RESULT " + JSON.stringify({ version: 1, phase, details, error: message(error) }));
  }
}

export default function App() {
  useEffect(() => { void observe().catch(error => console.error("INK_CONTRACT_RESULT " + JSON.stringify({ error: message(error) }))); }, []);
  return <Screen title="SDK contracts"><Text>Device observations are written to the test log.</Text></Screen>;
}
