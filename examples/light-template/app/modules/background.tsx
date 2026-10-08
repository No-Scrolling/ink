import { useEffect } from "react";
import { cancel, jobs } from "@ink/background";
import { Button, Field, Screen, Stack, Text, useAction, useSnapshot } from "ink";
import { backgroundResult, workerReport } from "../../data/background";
import { inspectWorker, refreshTodo } from "../../workers";

async function scheduleRefresh() {
  await refreshTodo.enqueue(null, {
    key: "example-todo",
    intervalMinutes: 15,
    constraints: { network: "connected" },
  });
}
export default function Background() {
  const schedule = useAction(scheduleRefresh);
  const cancelRefresh = useAction(() => cancel("example-todo"));
  const scheduledJobs = useSnapshot(jobs);
  const result = useSnapshot(backgroundResult);
  const report = useSnapshot(workerReport);
  const inspect = useAction(() => inspectWorker.enqueue(null, { key: "worker-inspection" }));
  useEffect(() => schedule.run(), [schedule.run]);
  const saved = result.status === "ready" ? result.data : null;
  const status = saved?.value
    ? saved.error
      ? "stale"
      : "ready"
    : saved?.error
      ? "error"
      : "waiting";

  return (
    <Screen title="Background">
      <Field label="Status">{status}</Field>
      <Button onPress={() => schedule.run()}>Schedule Refresh</Button>
      <Button onPress={() => cancelRefresh.run()}>Cancel Refresh</Button>
      <Button onPress={() => inspect.run()}>Run Worker APIs</Button>
      {inspect.status === "error" && <Text>{inspect.error.message}</Text>}
      {report.status === "ready" && <Text>{report.data}</Text>}
      {cancelRefresh.status === "error" && <Text>{cancelRefresh.error.message}</Text>}
      {scheduledJobs.status === "error" && <Text>{scheduledJobs.error.message}</Text>}
      {scheduledJobs.status === "ready" &&
        scheduledJobs.data.map((job) => (
          <Text key={job.key}>
            {job.key}: {job.status}
            {job.scheduled ? (job.periodic ? " (periodic)" : " (scheduled)") : ""}
          </Text>
        ))}
      {schedule.status === "error" && (
        <Field label="Schedule error">{schedule.error.message}</Field>
      )}
      {result.status === "error" ? (
        <Field label="Error">{result.error.message}</Field>
      ) : saved?.value ? (
        <Stack gap={16}>
          <Field label="Title">{saved.value.title}</Field>
          {saved.error && <Field label="Error">{saved.error}</Field>}
        </Stack>
      ) : (
        <Field label="Update">{saved?.error ?? "Waiting for the first update..."}</Field>
      )}
    </Screen>
  );
}
