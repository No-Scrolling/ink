import { expect, test } from "bun:test";
import { resolve } from "node:path";

test("development push server queues before waking, retries failed wakes and removes only acknowledged deliveries", async () => {
  let base = "";
  const wakes: { body: string; pending: unknown }[] = [];
  let failures = 1;
  const gateway = Bun.serve({
    hostname: "127.0.0.1",
    port: 0,
    async fetch(request) {
      wakes.push({
        body: await request.text(),
        pending: await (await fetch(base + "/phone")).json(),
      });
      if (failures-- > 0) return new Response("", { status: 500 });
      return new Response("", { status: 202 });
    },
  });
  const child = Bun.spawn(
    [process.execPath, resolve(import.meta.dir, "../../scripts/push-server.ts")],
    {
      env: { ...process.env, PORT: "0", PUSH_WAKE_RETRY_DELAY_MS: "1" },
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
    const pending = { messages: [{ version: 1, events: [event], id: first.queued }] };
    expect(wakes).toEqual([
      { body: "{}", pending },
      { body: "{}", pending },
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
    failures = 10;
    const wakeCount = wakes.length;
    const rejected = await request("/phone/send", "POST", event);
    expect(rejected.status).toBe(502);
    const rejectedWake = await rejected.json();
    expect(rejectedWake.wakeStatus).toBe(500);
    expect(wakes.slice(wakeCount).map((wake) => wake.body)).toEqual(["{}", "{}", "{}", "{}"]);
    expect((await (await request("/phone", "GET")).json()).messages).toContainEqual({
      version: 1, events: [event], id: rejectedWake.queued,
    });
    const endpoint = gateway.url.toString();
    gateway.stop(true);
    await request("/phone", "PUT", { endpoint });
    const failed = await request("/phone/send", "POST", event);
    expect(failed.status).toBe(502);
    const failedWake = await failed.json();
    expect(failedWake).toEqual({ queued: expect.any(String), wakeStatus: null });
    expect((await (await request("/phone", "GET")).json()).messages).toContainEqual({
      version: 1, events: [event], id: failedWake.queued,
    });
    for (const input of [null, [], 42, {}, { version: 1, events: [] }]) {
      expect((await request("/phone/send", "POST", input)).status).toBe(400);
    }
    expect((await fetch(base + "/phone/send", { method: "POST", body: "{" })).status).toBe(400);
    expect((await request("/phone", "PUT", { endpoint: "invalid" })).status).toBe(400);
    expect((await request("/phone", "POST", { ids: [42] })).status).toBe(400);
    expect(await (await request("/phone", "DELETE")).json()).toEqual({ success: true });
    expect((await request("/phone", "GET")).status).toBe(404);
  } finally {
    child.kill();
    await child.exited;
    gateway.stop(true);
  }
});
