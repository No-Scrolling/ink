import "@ink/network";
import { Screen, Text } from "ink";
import { useEffect } from "react";
import { createStore } from "@ink/store";
import { downloads } from "@ink/network/downloads";
import { callNative } from "ink/native";

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
      details.rustPackaging = await callNative("packaging", "probe", {});
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

      const manual = await fetch(`${origin}/redirect303`, { redirect: "manual" });
      details.manual = { status: manual.status, location: manual.headers.get("location"), redirected: manual.redirected };
      await manual.body?.cancel();
      try { await fetch(`${origin}/redirect303`, { redirect: "error" }); details.redirectError = "accepted"; }
      catch (error) { details.redirectError = message(error); }
      const missing = await fetch(`${origin}/missing`);
      details.missing = { status: missing.status, ok: missing.ok, text: await missing.text() };
      await fetch(`${origin}/set-cookie`);
      details.cookies = await (await fetch(`${origin}/cookies`)).text();
      details.crossOrigin = await (await fetch(`${origin}/cross-origin`, { headers: { authorization: "Bearer fixture", cookie: "fixture=1" } })).text();
      try { await fetch(`${origin}/delayed`, { signal: AbortSignal.timeout(100) }); details.timeout = "accepted"; }
      catch (error) { details.timeout = error instanceof Error ? error.name : String(error); }
      const streamedUpload = new ReadableStream<Uint8Array>({ start(controller) {
        controller.enqueue(new Uint8Array([0, 13, 255])); controller.enqueue(new Uint8Array([226, 130, 172])); controller.close();
      } });
      details.uploadStream = await (await fetch(`${origin}/stream-upload`, { method: "POST", body: streamedUpload })).text();

      let job = await downloads.enqueue({ key: "contract-file", url: `${origin}/download`, name: "fixture.bin" });
      const duplicate = await downloads.enqueue({ key: "contract-file", url: `${origin}/download`, name: "fixture.bin" });
      const deadline = Date.now() + 30_000;
      while (job.state !== "completed") {
        if (job.state === "failed" || Date.now() > deadline) throw new Error(`Download: ${job.error ?? job.state}`);
        await new Promise<void>(resolve => setTimeout(resolve, 100));
        job = await downloads.get(job.id);
      }
      const file = await (await fetch(job.file!.src)).blob();
      details.download = { duplicate: duplicate.id === job.id, received: job.received, total: job.total, bytes: Array.from(await file.bytes()) };
      const form = new FormData();
      form.append("caption", "café");
      form.append("attachment", file.slice(1, 5), "slice.bin");
      details.multipart = await (await fetch(`${origin}/multipart`, { method: "POST", body: form })).text();
      await downloads.remove(job.id);

      details.socket = await new Promise((resolve, reject) => {
        const socket = new WebSocket(`${origin.replace("http:", "ws:")}/socket`, "ink-contract");
        socket.binaryType = "arraybuffer";
        const messages: unknown[] = [];
        const timer = setTimeout(() => { socket.close(); reject(new Error("WebSocket timeout")); }, 15_000);
        socket.onopen = () => { socket.send("café"); socket.send(new Uint8Array([0, 255, 13, 10])); };
        socket.onmessage = event => {
          messages.push(typeof event.data === "string" ? event.data : Array.from(new Uint8Array(event.data as ArrayBuffer)));
          if (messages.length === 2) socket.close(1000, "Finished");
        };
        socket.onerror = () => { clearTimeout(timer); reject(new Error("WebSocket failed")); };
        socket.onclose = event => { clearTimeout(timer); resolve({ protocol: socket.protocol, messages, code: event.code, reason: event.reason, clean: event.wasClean }); };
      });
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
