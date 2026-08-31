import { cachedJson, json, mutation } from "@ink/network";
import { Button, Screen, Text, all, computed, match, routeParams, state } from "ink";

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
      <Text>{params.label}</Text>
      <Text>Computed: {doubled.value}</Text>
      {match(page, {
        loading: () => <Text>Loading resources...</Text>,
        ready: (result) => <Text>{result.value.first.title}</Text>,
        error: (result) => (
          <Text>{result.error.resource}: {result.error.error.message}</Text>
        ),
      })}
      {match(cached, {
        loading: () => <Text>Loading cache...</Text>,
        ready: (result) => <Text>{result.value.title}</Text>,
        stale: (result) => <Text>{result.value.title}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
      {match(save, {
        idle: () => <Button onPress={() => save.run()}>Run Mutation</Button>,
        running: () => <Text>Saving...</Text>,
        ready: (result) => <Text>Saved {result.value.id}</Text>,
        error: (result) => <Text>{result.error.message}</Text>,
      })}
    </Screen>
  );
}
