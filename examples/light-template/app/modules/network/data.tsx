import * as v from "valibot";
import { useEffect, useRef } from "react";
import { Button, Field, Screen, useAction, useRouteParams, useSnapshot } from "ink";
import { page, todo, saveTodo } from "../../../data/todos";

export default function Data() {
  const params = useRouteParams(v.parser(v.object({ label: v.fallback(v.string(), "") })));
  const count = 2;
  const result = useSnapshot(page());
  const source = todo(3);
  const cached = useSnapshot(source);
  const save = useAction(saveTodo);
  const session = useRef<AbortController | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    session.current = controller;
    return () => {
      controller.abort();
      session.current = null;
    };
  }, []);

  return (
    <Screen title="Data">
      <Field label="Route">{params.label}</Field>
      <Field label="Computed value">{count * 2}</Field>
      {result.status === "ready" ? (
        <Field label="First resource">{result.data.first.title}</Field>
      ) : (
        <Field label="Resources">
          {result.status === "error" ? result.error.message : "Loading..."}
        </Field>
      )}
      {cached.status === "ready" ? (
        <Field label={cached.refreshError ? "Stale cache" : "Cached resource"}>
          {cached.data.title}
        </Field>
      ) : (
        <Field label="Cache">
          {cached.status === "error" ? cached.error.message : "Loading..."}
        </Field>
      )}
      <Button
        onPress={() => {
          if (cached.status !== "loading" && !(cached.status === "ready" && cached.refreshing))
            source.refresh();
        }}
      >
        Refresh
      </Button>
      {save.status === "idle" ? (
        <Button onPress={() => save.run(count, session.current?.signal)}>Run Mutation</Button>
      ) : save.status === "pending" ? (
        <Field label="Mutation">Saving...</Field>
      ) : save.status === "success" ? (
        <Field label="Saved item">{save.data.id}</Field>
      ) : (
        <Field label="Mutation error">{save.error.message}</Field>
      )}
    </Screen>
  );
}
