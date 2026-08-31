import { cachedJson, json, mutation } from "@ink/network";
import { Button, Field, Screen, all, computed, match, routeParams, state } from "ink";

type Todo = {
  userId: number;
  id: number;
  title: string;
  completed: boolean;
  note?: string | null;
};

export default function Data() {
  const params = routeParams<{ label: string }>();
  const count = state(2);
  const doubled = computed(() => count.value * 2);
  const first = json<Todo>("https://jsonplaceholder.typicode.com/todos/1");
  const second = json<Todo>("https://jsonplaceholder.typicode.com/todos/2");
  const page = all({ first, second });
  const cached = cachedJson<Todo>("https://jsonplaceholder.typicode.com/todos/3");
  const save = mutation<Todo>("https://jsonplaceholder.typicode.com/todos", {
    method: "POST",
    body: { userId: count.value, title: "Ink", completed: false },
  });

  return (
    <Screen title="Data">
      <Field label="Route">{params.label}</Field>
      <Field label="Computed value">{doubled.value}</Field>
      {match(page, {
        loading: () => <Field label="Resources">Loading...</Field>,
        ready: (result) => (
          <Field label="First resource">{result.value.first.title}</Field>
        ),
        error: (result) => (
          <Field label="Resource error">
            {result.error.resource}: {result.error.error.message}
          </Field>
        ),
      })}
      {match(cached, {
        loading: () => <Field label="Cache">Loading...</Field>,
        ready: (result) => (
          <Field label="Cached resource">{result.value.title}</Field>
        ),
        stale: (result) => <Field label="Stale cache">{result.value.title}</Field>,
        error: (result) => <Field label="Cache error">{result.error.message}</Field>,
      })}
      {match(save, {
        idle: () => <Button onPress={() => save.run()}>Run Mutation</Button>,
        running: () => <Field label="Mutation">Saving...</Field>,
        ready: (result) => <Field label="Saved item">{result.value.id}</Field>,
        error: (result) => (
          <Field label="Mutation error">{result.error.message}</Field>
        ),
      })}
    </Screen>
  );
}
