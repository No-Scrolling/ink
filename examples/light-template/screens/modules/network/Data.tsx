import { useEffect, useRef, useState } from "react";
import { Button, Field, Screen, useAction, useRouteParams, useSnapshot } from "ink";
import { cachedTodo, loadPage, refreshCache, saveTodo } from "../../../data/todos";

export default function Data() {
  const params = useRouteParams<{ label: string }>();
  const [count] = useState(2);
  const page = useAction(loadPage);
  const refresh = useAction(refreshCache);
  const cached = useSnapshot(cachedTodo);
  const save = useAction(saveTodo);
  const session = useRef<AbortController | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    session.current = controller;
    page.run(controller.signal);
    refresh.run(controller.signal);
    return () => {
      controller.abort();
      session.current = null;
    };
  }, [page.run, refresh.run]);

  return (
    <Screen title="Data">
      <Field label="Route">{params.label}</Field>
      <Field label="Computed value">{count * 2}</Field>
      {page.status === "success" ? (
        <Field label="First resource">{page.data.first.title}</Field>
      ) : <Field label="Resources">{page.status === "error" ? page.error.message : "Loading..."}</Field>}
      {cached.status === "ready" && cached.data !== null ? (
        <Field label={refresh.status === "error" ? "Stale cache" : "Cached resource"}>{cached.data.title}</Field>
      ) : <Field label="Cache">{cached.status === "error" ? cached.error.message : refresh.status === "error" ? refresh.error.message : "Loading..."}</Field>}
      {save.status === "idle" ? (
        <Button onPress={() => save.run(count, session.current?.signal)}>Run Mutation</Button>
      ) : save.status === "pending" ? (
        <Field label="Mutation">Saving...</Field>
      ) : save.status === "success" ? (
        <Field label="Saved item">{save.data.id}</Field>
      ) : <Field label="Mutation error">{save.error.message}</Field>}
    </Screen>
  );
}
