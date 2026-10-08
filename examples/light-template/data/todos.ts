import * as v from "valibot";
import "@ink/network";
import { resource } from "ink";

export const todoSchema = v.object({
  userId: v.pipe(v.number(), v.safeInteger()),
  id: v.pipe(v.number(), v.safeInteger()),
  title: v.string(),
  completed: v.boolean(),
  note: v.optional(v.nullable(v.string())),
});
export type Todo = v.InferOutput<typeof todoSchema>;
export const decodeTodo = v.parser(todoSchema);
async function readTodo(id: number, signal?: AbortSignal) {
  const response = await fetch(`https://jsonplaceholder.typicode.com/todos/${id}`, { signal });
  if (!response.ok) throw new Error(`Todo ${id}: HTTP ${response.status}`);
  return decodeTodo(await response.json());
}
export const todo = resource({
  key: (id: number) => [id],
  load: (id) => readTodo(id),
  staleTime: 60_000,
});
export const page = resource({
  key: () => [],
  load: async () => {
    const [first, second] = await Promise.all([readTodo(1), readTodo(2)]);
    return { first, second };
  },
  staleTime: 60_000,
});
export async function saveTodo(userId: number, signal?: AbortSignal) {
  const response = await fetch("https://jsonplaceholder.typicode.com/todos", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ userId, title: "Ink", completed: false }),
    signal,
  });
  if (!response.ok) throw new Error(`Could not save todo: HTTP ${response.status}`);
  return decodeTodo(await response.json());
}
