import { periodicJson } from "@ink/background";
import { Screen, Text } from "ink";

type Todo = {
  userId: number;
  id: number;
  title: string;
  completed: boolean;
};

export default function Background() {
  const todo = periodicJson<Todo>(
    "example-todo",
    "https://jsonplaceholder.typicode.com/todos/1",
    { everyMinutes: 15 },
  );

  return (
    <Screen title="Background">
      <Text>Status: {todo.status}</Text>
      {todo.status === "ready" && <Text>{todo.value.title}</Text>}
      {todo.status === "stale" && <Text>{todo.value.title}</Text>}
      {todo.status === "stale" && <Text>{todo.error.message}</Text>}
      {todo.status === "error" && <Text>{todo.error.message}</Text>}
    </Screen>
  );
}
