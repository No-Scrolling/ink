import { periodicJson } from "@ink/background";
import { Field, Screen, Stack, match } from "ink";

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
      <Field label="Status">{todo.status}</Field>
      {match(todo, {
        waiting: () => <Field label="Update">Waiting for the first update...</Field>,
        ready: (result) => <Field label="Title">{result.value.title}</Field>,
        stale: (result) => (
          <Stack gap={16}>
            <Field label="Title">{result.value.title}</Field>
            <Field label="Error">{result.error.message}</Field>
          </Stack>
        ),
        error: (result) => <Field label="Error">{result.error.message}</Field>,
      })}
    </Screen>
  );
}
