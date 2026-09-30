import "@ink/network";
import { Screen, Text } from "ink";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;
const report = (label: string, value: unknown) =>
  __inkPost(JSON.stringify({ type: "probe", label, value }));

onNativeMessage("test", message => {
  void inspect(String(message.command)).catch(error => report("error", String(error)));
});

async function inspect(command: string) {
  if (command === "status") {
    report("status", "running");
  } else if (command === "url") {
    const url = new URL("../café?tag=one&tag=two", "https://EXAMPLE.com/notes/");
    report("url", [url.href, url.hostname, url.searchParams.getAll("tag")]);
  } else if (command === "bodies") {
    const blob = new Blob([new Uint8Array([0, 128, 255]), "café"]);
    const response = new Response('{"title":"café"}', {
      status: 201, headers: { "Content-Type": "application/json" },
    });
    const clone = response.clone();
    report("bodies", [
      Array.from(new Uint8Array(await blob.arrayBuffer())),
      response.status, response.headers.get("content-type"),
      await response.json(), await clone.text(), response.bodyUsed, clone.bodyUsed,
    ]);
  } else if (command === "streams") {
    const stream = new ReadableStream<string>({
      start(controller) { controller.enqueue("café"); controller.enqueue("notes"); controller.close(); },
    }).pipeThrough(new TransformStream<string, string>({
      transform(value, controller) { controller.enqueue(value.toUpperCase()); },
    }));
    const reader = stream.getReader();
    report("streams", [await reader.read(), await reader.read(), await reader.read()]);
  }
}

export default function App() {
  return <Screen><Text>Split web contract</Text></Screen>;
}
