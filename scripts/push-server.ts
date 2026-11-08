// Local development only: subscriptions and queued envelopes live in memory.
const subscriptions = new Map<string, { endpoint: string; messages: { id: string }[] }>();
const base = "/v1/push/subscriptions/";
const port = Number(process.env.PORT ?? 18080);
const retryDelay = Number(process.env.PUSH_WAKE_RETRY_DELAY_MS ?? 1000);

// Light's gateway sometimes returns 5xx; retry after 1, 2 and 4 seconds.
async function sendWake(endpoint: string) {
  for (const delay of [retryDelay, retryDelay * 2, retryDelay * 4, 0]) {
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
      if (!delay) {
        console.error(error);
        return null;
      }
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
    let input: Record<string, unknown> = {};
    if (request.method === "POST" || request.method === "PUT") {
      try {
        const value: Record<string, unknown> = await request.json();
        if (value === null || typeof value !== "object" || Array.isArray(value)) {
          return new Response("Expected a JSON object", { status: 400 });
        }
        input = value;
      } catch (error) {
        if (!(error instanceof SyntaxError)) throw error;
        return new Response("Invalid JSON", { status: 400 });
      }
    }
    if (request.method === "POST" && path.endsWith("/send")) {
      const subscription = subscriptions.get(id);
      if (!subscription) return new Response("Unknown subscription", { status: 404 });
      // Accept a complete version-1 envelope, or one show/clear event.
      const envelope = input.version === 1 ? input : { version: 1, events: [input] };
      if (!Array.isArray(envelope.events) || !envelope.events.length || !envelope.events.every(
        (event: unknown) => event !== null && typeof event === "object" && "operation" in event &&
          (event.operation === "show" || event.operation === "clear"),
      )) {
        return new Response("Expected an event or version-1 envelope", { status: 400 });
      }
      const receipt = crypto.randomUUID();
      subscription.messages.push({ ...envelope, id: receipt });
      const wake = await sendWake(subscription.endpoint);
      const wakeStatus = wake?.status ?? null;
      console.log(`Wake ${wakeStatus ?? "failed"}; queued ${receipt} for ${id}`);
      return Response.json(
        { queued: receipt, wakeStatus },
        { status: wake?.ok ? 200 : 502 },
      );
    }
    if (request.method === "PUT") {
      const { endpoint } = input;
      if (typeof endpoint !== "string" || !URL.canParse(endpoint)) {
        return new Response("Expected a valid endpoint URL", { status: 400 });
      }
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
      const { ids } = input;
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
  },
  error(error) {
    console.error(error);
    return new Response("Internal server error", { status: 500 });
  },
});
console.log(`Push subscription server: ${server.url}v1/push/subscriptions`);
