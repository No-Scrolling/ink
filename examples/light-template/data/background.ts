import { createStore } from "@ink/store";
import { decodeTodo, type Todo } from "./todos";

type BackgroundResult = { value: Todo | null; updatedAt: number; error: string | null };
export const backgroundResult = createStore<BackgroundResult>({
  key: "template.background-todo", version: 1,
  initial: { value: null, updatedAt: 0, error: null },
  decode(input) {
    if (typeof input !== "object" || input === null || !("value" in input)
      || !("updatedAt" in input) || typeof input.updatedAt !== "number" || !Number.isSafeInteger(input.updatedAt) || input.updatedAt < 0
      || !("error" in input) || (input.error !== null && typeof input.error !== "string")) throw new Error("Invalid saved background result");
    return { value: input.value === null ? null : decodeTodo(input.value), updatedAt: input.updatedAt, error: input.error };
  },
});

export const pushResult = createStore({
  key: "template.background-push", version: 1,
  initial: { deliveries: 0, message: "No background push yet" },
  decode(input: unknown) {
    if (typeof input !== "object" || input === null || !("deliveries" in input) || typeof input.deliveries !== "number"
      || !("message" in input) || typeof input.message !== "string") throw new Error("Invalid push result");
    return { deliveries: input.deliveries, message: input.message };
  },
});

export const workerReport = createStore({
  key: "template.worker-report", version: 1, initial: "Worker APIs have not run yet",
  decode(value: unknown) {
    if (typeof value !== "string") throw new Error("Invalid worker report");
    return value;
  },
});
