import * as v from "valibot";
import { createStore } from "@ink/store";
import { todoSchema } from "./todos";

const resultSchema = v.object({ value: v.nullable(todoSchema), updatedAt: v.pipe(v.number(), v.safeInteger(), v.minValue(0)), error: v.nullable(v.string()) });
export const backgroundResult = createStore<v.InferOutput<typeof resultSchema>>({
  key: "template.background-todo", version: 1,
  initial: { value: null, updatedAt: 0, error: null },
  decode: v.parser(resultSchema),
});

export const pushResult = createStore({
  key: "template.background-push", version: 1,
  initial: { deliveries: 0, message: "No background push yet" },
  decode: v.parser(v.object({ deliveries: v.number(), message: v.string() })),
});

export const workerReport = createStore({
  key: "template.worker-report", version: 1, initial: "Worker APIs have not run yet",
  decode: v.parser(v.string()),
});
