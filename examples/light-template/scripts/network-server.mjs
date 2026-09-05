const server = Bun.serve({
  hostname: "127.0.0.1",
  port: 18081,
  async fetch(request, server) {
    const url = new URL(request.url);
    if (url.pathname === "/socket") {
      if (server.upgrade(request)) return undefined;
      return new Response("upgrade required", { status: 426 });
    }
    console.log(request.method, url.pathname);
    if (url.pathname === "/stream") {
      let index = 0;
      return new Response(new ReadableStream({
        async pull(controller) {
          if (index === 96) { controller.close(); return; }
          await Bun.sleep(10);
          controller.enqueue(new Uint8Array(32768).fill(index++ % 251));
        },
        cancel() { console.log("stream cancelled at chunk", index); },
      }), { headers: { "content-type": "application/octet-stream" } });
    }
    if (url.pathname === "/slow") {
      return new Response(new ReadableStream({
        async pull(controller) { await Bun.sleep(3000); controller.enqueue(new TextEncoder().encode("slow\n")); },
        cancel() { console.log("slow stream cancelled"); },
      }));
    }
    if (url.pathname === "/multipart") {
      const form = await request.formData();
      const photo = form.get("file");
      return Response.json({ name: form.get("name"), size: photo instanceof File ? photo.size : null, bytes: photo instanceof File ? Array.from(new Uint8Array(await photo.arrayBuffer()).slice(0, 8)) : null });
    }
    if (url.pathname === "/echo") {
      const bytes = new Uint8Array(await request.arrayBuffer());
      return Response.json({ length: bytes.length, first: bytes[0], last: bytes.at(-1), method: request.method });
    }
    if (url.pathname === "/redirect") return Response.redirect("http://127.0.0.1:18081/echo",307);
    return new Response("ok");
  },
  websocket: {
    open() { console.log("socket opened"); },
    message(socket, message) { socket.send(message); },
    close(socket, code, reason) { console.log("socket closed", code, reason); },
  },
});
console.log(`Ink network harness ${server.url}`);
