import { Screen, Text } from "ink";
import { callNative, callNativeBytes, NativeError, onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;
const report = (label: string, value: unknown) =>
  __inkPost(JSON.stringify({ type: "probe", label, value }));
let controller: AbortController | undefined;
let retained: Uint8Array | undefined;
onNativeMessage("test", (message) => {
  const command = message.command;
  if (command === "call") {
    controller = new AbortController();
    if (message.preAborted) controller.abort("before-send");
    callNative(
      "fixture",
      "lookup",
      { name: "café" },
      { signal: controller.signal, timeoutMs: 1234 },
    ).then(
      (value) => report("settled", value),
      (error) =>
        report(
          "rejected",
          error instanceof NativeError ? [error.kind, error.message, error.retryable] : error,
        ),
    );
  } else if (command === "abort") {
    controller?.abort("user-cancelled");
    report("aborted", true);
  } else if (command === "binary") {
    const source = new Uint8Array([99, 0, 127, 128, 255, 88]);
    callNativeBytes("fixture", "binary", {}, { bytes: source.subarray(1, 5) }).then((result) => {
      retained = result.bytes;
      report("binary-result", [result.value, Array.from(retained)]);
    });
    source.fill(9);
  } else if (command === "retained") {
    report("retained", Array.from(retained ?? []));
  } else if (command === "pending") {
    for (let index = 0; index < Number(message.count); index++) {
      callNative("fixture", "pending", { index }).catch((error) =>
        report("capacity", [error.kind, error.retryable]),
      );
    }
    queueMicrotask(() => report("pending-sent", message.round));
  } else if (command === "echo") report("echo", message.value);
});
export default function App() {
  return (
    <Screen>
      <Text>Runtime fixture</Text>
    </Screen>
  );
}
