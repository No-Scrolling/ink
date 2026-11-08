import { expect, test } from "bun:test";
import { resolve } from "node:path";

test("development push server queues before waking and removes only acknowledged deliveries", async () => {
  let base = "";
  const wakes: { body: string; pending: unknown }[] = [];
  const gateway = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    async fetch(request) {
      wakes.push({
        body: await request.text(),
        pending: await (await fetch(base + "/phone")).json(),
      });
      return new Response("", { status: 202 });
    },
  });
  const child = Bun.spawn(
    [process.execPath, resolve(import.meta.dir, "../../scripts/push-server.ts")],
    {
      env: { ...process.env, PORT: "0" },
      stdout: "pipe",
      stderr: "pipe",
    },
  );
  try {
    const reader = child.stdout.getReader();
    let output = "";
    while (!output.includes("\n")) {
      const chunk = await reader.read();
      if (chunk.done) throw new Error("Push server exited before listening");
      output += new TextDecoder().decode(chunk.value);
    }
    reader.releaseLock();
    base = output.trim().replace("Push subscription server: ", "");
    const request = (path: string, method: string, body?: unknown) =>
      fetch(base + path, {
        method,
        headers: { "Content-Type": "application/json" },
        body: body === undefined ? undefined : JSON.stringify(body),
      });
    expect(
      await (await request("/phone", "PUT", { endpoint: gateway.url.toString() })).json(),
    ).toEqual({ success: true });
    const event = {
      operation: "show",
      id: "event-1",
      groupKey: "thread-1",
      title: "Test",
      body: "Inbox delivery",
      route: "/inbox",
    };
    const first = await (await request("/phone/send", "POST", event)).json();
    expect(first.wakeStatus).toBe(202);
    expect(wakes).toEqual([
      { body: "{}", pending: { messages: [{ version: 1, events: [event], id: first.queued }] } },
    ]);
    const clear = {
      version: 1,
      events: [{ operation: "clear", id: "event-2", groupKey: "thread-1" }],
    };
    const second = await (await request("/phone/send", "POST", clear)).json();
    expect(second.queued).not.toBe(first.queued);
    expect(
      await (await request("/phone", "PUT", { endpoint: gateway.url.toString() })).json(),
    ).toEqual({ success: true });
    expect(await (await request("", "GET")).json()).toEqual([{ id: "phone", queued: 2 }]);
    expect(await (await request("/phone", "POST", { ids: [first.queued] })).json()).toEqual({
      success: true,
    });
    expect(await (await request("/phone", "GET")).json()).toEqual({
      messages: [{ ...clear, id: second.queued }],
    });
    expect(await (await request("/phone", "DELETE")).json()).toEqual({ success: true });
    expect((await request("/phone", "GET")).status).toBe(404);
  } finally {
    child.kill();
    await child.exited;
    gateway.stop(true);
  }
});
