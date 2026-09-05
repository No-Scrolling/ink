import "ink/runtime";
import { onNativeMessage } from "ink/native";
import { runRegisteredTask } from "./index";

declare const __inkPost: (message: string) => void;
export function startWorker() {
  const controller = new AbortController();
  let started = false;
  onNativeMessage("cancel-task", () => controller.abort());
  onNativeMessage("run-task", message => {
    if (started || typeof message.task !== "string") throw new Error("Invalid worker invocation");
    started = true;
    void runRegisteredTask(message.task, message.input, controller.signal).then(
      result => __inkPost(JSON.stringify({ type: "task-result", result })),
      error => {
        console.error("Background task failed", error);
        __inkPost(JSON.stringify({ type: "task-result", result: { status: "failed", reason: error instanceof Error ? error.name : "unexpected" } }));
      },
    );
  });
  __inkPost(JSON.stringify({ type: "worker-ready" }));
}
