// Local development only: subscriptions and queued envelopes live in memory.
const subscriptions = new Map<string, { endpoint: string; messages: Record<string, unknown>[] }>();
const base = "/v1/push/subscriptions/";
const port = Number(process.env.PORT ?? 18080);

// Light's gateway sometimes returns 5xx; retry after 1, 2 and 4 seconds.
async function sendWake(endpoint: string) {
  for (const delay of [1_000, 2_000, 4_000, 0]) {
    try {
      const wake = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: "{}",
        signal: AbortSignal.timeout(10_000),
      });
      if (wake.status < 500 || !delay) return wake;
      console.log(`Wake ${wake.status}; retrying in ${delay / 1000}s`);
    } catch (error) {
      if (!delay) throw error;
      console.log(`Wake failed; retrying in ${delay / 1000}s`);
    }
    await Bun.sleep(delay);
  }
  throw new Error("unreachable");
}
const server = Bun.serve({
  hostname: "127.0.0.1",
  port,
  async fetch(request) {
    const path = new URL(request.url).pathname;
    if (path === "/v1/push/subscriptions" && request.method === "GET") {
      return Response.json(
        [...subscriptions].map(([id, value]) => ({ id, queued: value.messages.length })),
      );
    }
    if (!path.startsWith(base)) return new Response("Not found", { status: 404 });
    const id = path.slice(base.length).replace(/\/send$/, "");
    if (!id || id.includes("/")) return new Response("Invalid installation ID", { status: 400 });
    try {
      if (request.method === "POST" && path.endsWith("/send")) {
        const subscription = subscriptions.get(id);
        if (!subscription) return new Response("Unknown subscription", { status: 404 });
        // Accept a complete version-1 envelope, or one show/clear event.
        const input = await request.json();
        const envelope = input.version === 1 ? input : { version: 1, events: [input] };
        if (!Array.isArray(envelope.events) || !envelope.events.length) {
          return new Response("Expected an event or version-1 envelope", { status: 400 });
        }
        const receipt = crypto.randomUUID();
        subscription.messages.push({ ...envelope, id: receipt });
        const wake = await sendWake(subscription.endpoint);
        console.log(`Wake ${wake.status}; queued ${receipt} for ${id}`);
        return Response.json(
          { queued: receipt, wakeStatus: wake.status },
          { status: wake.ok ? 200 : 502 },
        );
      }
      if (request.method === "PUT") {
        const { endpoint } = await request.json();
        const url = new URL(endpoint);
        if (
          url.protocol !== "https:" &&
          !(url.protocol === "http:" && ["localhost", "127.0.0.1"].includes(url.hostname))
        ) {
          return new Response("Expected an HTTPS endpoint or local HTTP endpoint", { status: 400 });
        }
        subscriptions.set(id, { endpoint, messages: subscriptions.get(id)?.messages ?? [] });
        console.log(`Registered ${id}`);
      } else if (request.method === "DELETE") {
        subscriptions.delete(id);
        console.log(`Unregistered ${id}`);
      } else {
        const subscription = subscriptions.get(id);
        if (!subscription) return new Response("Unknown subscription", { status: 404 });
        if (request.method === "GET") return Response.json({ messages: subscription.messages });
        if (request.method !== "POST") return new Response("Method not allowed", { status: 405 });
        const { ids } = await request.json();
        if (!Array.isArray(ids) || !ids.every((id) => typeof id === "string")) {
          return new Response("Expected receipt IDs", { status: 400 });
        }
        subscription.messages = subscription.messages.filter(
          (message) => !ids.includes(message.id),
        );
        console.log(
          `Acknowledged ${ids.length} for ${id}; ${subscription.messages.length} pending`,
        );
      }
      return Response.json({ success: true });
    } catch (error) {
      console.error(error);
      return new Response("Invalid request or failed wake; queued events remain available", {
        status: 400,
      });
    }
  },
});
console.log(`Push subscription server: ${server.url}v1/push/subscriptions`);
