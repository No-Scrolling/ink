import { periodicJson } from "@ink/background";
import { Screen, Stack, Text, match } from "ink";

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
      {match(todo, {
        waiting: () => <Text>Waiting for the first update...</Text>,
        ready: (result) => <Text>{result.value.title}</Text>,
        stale: (result) => (
          <Stack gap={16}>
            <Text>{result.value.title}</Text>
            <Text>{result.error.message}</Text>
          </Stack>
        ),
        error: (result) => <Text>{result.error.message}</Text>,
      })}
    </Screen>
  );
}
