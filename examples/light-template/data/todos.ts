import "@ink/network";
import { resource } from "ink";

export interface Todo {
  userId: number;
  id: number;
  title: string;
  completed: boolean;
  note?: string | null;
}
export function decodeTodo(value: unknown): Todo {
  if (typeof value !== "object" || value === null
    || !("userId" in value) || typeof value.userId !== "number" || !Number.isSafeInteger(value.userId)
    || !("id" in value) || typeof value.id !== "number" || !Number.isSafeInteger(value.id)
    || !("title" in value) || typeof value.title !== "string"
    || !("completed" in value) || typeof value.completed !== "boolean") throw new Error("Invalid todo response");
  const todo: Todo = { userId: value.userId, id: value.id, title: value.title, completed: value.completed };
  if ("note" in value) {
    if (value.note !== null && typeof value.note !== "string") throw new Error("Invalid todo note");
    todo.note = value.note;
  }
  return todo;
}
async function readTodo(id: number, signal?: AbortSignal) {
  const response = await fetch(`https://jsonplaceholder.typicode.com/todos/${id}`, { signal });
  if (!response.ok) throw new Error(`Todo ${id}: HTTP ${response.status}`);
  return decodeTodo(await response.json());
}
export const todo = resource({
  key: (id: number) => [id],
  load: id => readTodo(id),
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
    method: "POST", headers: { "content-type": "application/json" },
    body: JSON.stringify({ userId, title: "Ink", completed: false }), signal,
  });
  if (!response.ok) throw new Error(`Could not save todo: HTTP ${response.status}`);
  return decodeTodo(await response.json());
}
