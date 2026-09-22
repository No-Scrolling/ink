import * as v from "valibot";
import "@ink/network";
export async function streamResponse(base: string, signal: AbortSignal) {
  const response = await fetch(`${base}/stream`, { signal });
  if (!response.ok || !response.body) throw new Error(`HTTP ${response.status}`);
  const reader = response.body.getReader();
  let size = 0, checksum = 0, chunks = 0;
  try {
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      size += value.length;
      for (const byte of value) checksum += byte;
      chunks++;
    }
  } finally { reader.releaseLock(); }
  return `${size} bytes, ${chunks} chunks, checksum ${checksum}`;
}

export async function uploadForm(base: string, signal: AbortSignal) {
  const form = new FormData();
  form.append("name", "Ink café");
  form.append("file", new Blob([new Uint8Array(1024 * 1024).fill(37)], { type: "application/octet-stream" }), "sample.bin");
  const response = await fetch(`${base}/multipart`, { method: "POST", body: form, signal });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return response.text();
}

export async function cancelResponse(base: string, signal: AbortSignal) {
  const controller = new AbortController();
  const response = await fetch(`${base}/stream`, { signal: AbortSignal.any([signal, controller.signal]) });
  const reader = response.body?.getReader();
  if (!reader) throw new Error("No response stream");
  try {
    await reader.read();
    controller.abort();
    try { await reader.read(); }
    catch (error) {
      if (error instanceof Error && error.name === "AbortError") return "Cancelled after the first chunk";
      throw error;
    }
    throw new Error("Cancelled response continued reading");
  } finally { reader.releaseLock(); }
}

export function echoSocket(base: string, signal: AbortSignal): Promise<string> {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(`${base.replace(/^http/, "ws")}/socket`);
    socket.binaryType = "arraybuffer";
    const abort = () => { socket.close(); reject(signal.reason); };
    signal.addEventListener("abort", abort, { once: true });
    if (signal.aborted) abort();
    let messages = 0;
    socket.onopen = () => {
      socket.send("Ink café");
      socket.send(new Uint8Array([0, 1, 255]));
    };
    socket.onmessage = event => {
      const valid = messages === 0 ? event.data === "Ink café"
        : event.data instanceof ArrayBuffer && new Uint8Array(event.data).join(",") === "0,1,255";
      if (!valid) { reject(new Error("Unexpected WebSocket echo")); socket.close(); return; }
      if (++messages === 2) socket.close(1000, "Finished");
    };
    socket.onerror = () => reject(new Error("WebSocket connection failed"));
    socket.onclose = event => {
      signal.removeEventListener("abort", abort);
      if (messages === 2 && event.wasClean) resolve("Text and binary echoed; closed cleanly");
      else reject(new Error(`WebSocket closed (${event.code})`));
    };
  });
}

export async function replayUpload(base: string, signal: AbortSignal) {
  const request = new Request(`${base}/redirect`, {
    method: "POST", body: new Blob([new Uint8Array(1024 * 1024).fill(7)]), signal,
  });
  const response = await fetch(request);
  if (!response.ok) throw new Error(`Upload failed (HTTP ${response.status})`);
  v.parse(v.object({ length: v.literal(1024 * 1024), first: v.literal(7), last: v.literal(7), method: v.literal("POST") }), await response.json());
  if (!request.bodyUsed || !response.redirected) throw new Error("Upload was not redirected");
  return "1 MiB POST replayed after HTTP 307; original request consumed";
}
