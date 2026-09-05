import { callNative, NativeError, onNativeMessage } from "./native";

const observers = new Map<number, (value: unknown) => void>();
let nextId = 1;

onNativeMessage("controller", message => {
  if (typeof message.id !== "number") throw new NativeError("protocol", "Invalid controller event ID");
  observers.get(message.id)?.(message.value);
});

export function attachNativeController(
  module: "audio" | "notifications" | "camera",
  recipe: unknown,
  observe: (value: unknown) => void,
) {
  if (observers.size >= 256) throw new NativeError("busy", "Too many native controllers");
  const id = nextId++;
  let disposed = false;
  observers.set(id, observe);
  const ready = callNative(module, "activate", recipe, { controller: id }).catch(error => {
    observers.delete(id);
    throw error;
  });
  return {
    id,
    ready,
    call(operation: string, payload: unknown = {}) {
      if (disposed) return Promise.reject(new NativeError("unavailable", "Native controller is disposed"));
      return ready.then(() => {
        if (disposed) throw new NativeError("unavailable", "Native controller is disposed");
        return callNative(module, operation, payload, { controller: id });
      });
    },
    async dispose() {
      if (disposed) return;
      disposed = true;
      observers.delete(id);
      await callNative(module, "deactivate", {}, { controller: id });
    },
  };
}
