import { expect, test } from "bun:test";

const requests: { message: string; bytes?: Uint8Array }[] = [];
let next = 1;
Object.assign(globalThis, {
  __inkNextId: () => next++,
  __inkPost: (message: string) => requests.push({ message }),
  __inkPostBytes: (message: string, bytes: Uint8Array) => requests.push({ message, bytes: bytes.slice() }),
});
const { callNative, callNativeBytes } = await import("../../packages/ink/src/native");
const receive = Reflect.get(globalThis, "__inkReceive") as (source: string, bytes?: Uint8Array) => void;

test("binary replies preserve data and ordinary string calls", async () => {
  const pending = callNativeBytes("network", "stream-read-bytes", {});
  const request = JSON.parse(requests.pop()!.message);
  receive(JSON.stringify({ type: "result", id: request.id, value: '{"done":false}' }), new Uint8Array([0, 128, 255]));
  const result = await pending;
  expect([...result.bytes]).toEqual([0, 128, 255]);
  expect(JSON.parse(result.value)).toEqual({ done: false });
  const plain = callNative("store", "get", {});
  receive(JSON.stringify({ type: "result", id: JSON.parse(requests.pop()!.message).id, value: "ordinary" }));
  expect(await plain).toBe("ordinary");
});

test("cancelled binary calls ignore late results and retain timeout/error behaviour", async () => {
  const abort = new AbortController();
  const pending = callNativeBytes("network", "stream-upload-write", {}, { signal: abort.signal, bytes: new Uint8Array([255, 0]) });
  const request = requests.pop()!;
  expect([...request.bytes!]).toEqual([255, 0]);
  const id = JSON.parse(request.message).id;
  abort.abort(new Error("cancelled"));
  expect(JSON.parse(requests.pop()!.message)).toEqual({ type: "cancel", id });
  await expect(pending).rejects.toThrow("cancelled");
  receive(JSON.stringify({ type: "result", id, value: "late" }), new Uint8Array([1]));
  const failed = callNativeBytes("network", "stream-read-bytes", {});
  receive(JSON.stringify({ type: "result", id: JSON.parse(requests.pop()!.message).id, kind: "timeout", message: "Timed out", retryable: true }));
  await expect(failed).rejects.toMatchObject({ kind: "timeout", retryable: true });
});

test("managed Blob slices read their exact byte range through binary transport", async () => {
  const { Blob } = await import("../../packages/network/src/blob");
  const blob = Blob.fromNative("ink-file://fixture", 40_000, "application/octet-stream");
  const pending = blob.slice(32_760, 32_780).bytes();
  await Bun.sleep(0);
  const request = JSON.parse(requests.pop()!.message);
  expect(request.operation).toBe("stream-file-read-bytes");
  expect(request.payload).toEqual({ src: "ink-file://fixture", offset: 32_760, size: 20 });
  const bytes = Uint8Array.from({ length: 20 }, (_, index) => 236 + index);
  receive(JSON.stringify({ type: "result", id: request.id, value: "" }), bytes);
  expect(await pending).toEqual(bytes);
  expect(blob.size).toBe(40_000);
});

test("custom playback observers publish changes and discard events after disposal", async () => {
  const { attachNativeController } = await import("../../packages/ink/src/controller");
  const snapshots: unknown[] = [];
  const controller = attachNativeController("app-player", {}, value => snapshots.push(value));
  const activation = JSON.parse(requests.pop()!.message);
  expect(activation.module).toBe("app-player");
  receive(JSON.stringify({ type: "result", id: activation.id, value: "" }));
  await controller.ready;
  receive(JSON.stringify({ type: "controller", id: controller.id, value: { position: 100, playing: true } }));
  const read = controller.call("getState");
  await Bun.sleep(0);
  const request = JSON.parse(requests.pop()!.message);
  receive(JSON.stringify({ type: "result", id: request.id, value: '{"position":1500,"playing":true}' }));
  expect(JSON.parse(await read).position).toBe(1500);
  expect(snapshots).toEqual([{ position: 100, playing: true }]);
  const dispose = controller.dispose();
  await Bun.sleep(0);
  const release = JSON.parse(requests.pop()!.message);
  expect(release.operation).toBe("deactivate");
  receive(JSON.stringify({ type: "controller", id: controller.id, value: { position: 2000 } }));
  receive(JSON.stringify({ type: "result", id: release.id, value: "" }));
  await dispose;
  expect(snapshots).toHaveLength(1);
  await expect(controller.call("getState")).rejects.toMatchObject({ kind: "unavailable" });
});
