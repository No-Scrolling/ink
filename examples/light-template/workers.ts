import * as v from "valibot";
import { lightos } from "@ink/lightos";
import "@ink/network";
import { notifications } from "@ink/notifications";
import { location } from "@ink/location";
import { defineTask, jobs } from "@ink/background";
import { decodePushDelivery } from "@ink/lightos/push";
import { backgroundResult, pushResult, workerReport } from "./data/background";
import { decodeTodo } from "./data/todos";

export const refreshTodo = defineTask({
  id: "template.refresh-todo",
  decode: v.parser(v.null()),
  async run({ signal }) {
    try {
      const response = await fetch("https://jsonplaceholder.typicode.com/todos/1", { signal });
      if (!response.ok) throw new Error(`Background refresh: HTTP ${response.status}`);
      const value = decodeTodo(await response.json());
      signal.throwIfAborted();
      await backgroundResult.set({ value, updatedAt: Date.now(), error: null });
      return { status: "success" };
    } catch (error) {
      signal.throwIfAborted();
      await backgroundResult.update((previous) => ({
        ...previous,
        error: error instanceof Error ? error.message : String(error),
      }));
      return { status: "retry", delayMs: 60_000 };
    }
  },
});

export const processPush = defineTask({
  id: "template.process-push",
  decode: decodePushDelivery,
  async run({ input, signal }) {
    signal.throwIfAborted();
    await pushResult.update((previous) => ({
      deliveries: previous.deliveries + 1,
      message:
        input.messages.map((message) => message.title).join(", ") ||
        `${input.cancelled.length} groups cleared`,
    }));
    return { status: "success" };
  },
});

export const inspectWorker = defineTask({
  id: "template.inspect-worker",
  decode: v.parser(v.null()),
  async run({ signal }) {
    const permission = await lightos.getPermission("notifications");
    const exact = await lightos.canScheduleExact();
    const scheduledJobs = await jobs.get({ signal });
    if (permission === "granted") {
      await notifications.show({
        id: "worker-inspection",
        title: "Background worker",
        body: "Native notifications are available",
      });
      await notifications.cancel("worker-inspection");
    }
    let position: string;
    try {
      const fix = await location.current({ signal, timeout: 2_000 });
      position = `${fix.latitude}, ${fix.longitude}`;
    } catch (error) {
      signal.throwIfAborted();
      position = error instanceof Error ? error.message : String(error);
    }
    await workerReport.set(
      `Notifications: ${permission}. Exact alarms: ${exact}. Jobs: ${scheduledJobs.length}. Location: ${position}`,
    );
    return { status: "success" };
  },
});
