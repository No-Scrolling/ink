import { useEffect, useRef, useState } from "react";
import { Button, Screen, Text, TextInput, useAction } from "ink";
import { cancelResponse, echoSocket, replayUpload, streamResponse, uploadForm } from "../../../data/network-features";

export default function NetworkFeatures() {
  const [server, setServer] = useState("http://127.0.0.1:18081");
  const active = useRef<AbortController | null>(null);
  const request = useAction(async (operation: typeof streamResponse) => {
    active.current?.abort();
    const controller = new AbortController();
    active.current = controller;
    return operation(server.replace(/\/$/, ""), AbortSignal.any([controller.signal, AbortSignal.timeout(30_000)]));
  });
  useEffect(() => () => active.current?.abort(), []);
  return (
    <Screen title="Network Features">
      <Text>Run the network example server on your computer and forward port 18081.</Text>
      <TextInput value={server} onChange={setServer} action="done" />
      <Button onPress={() => request.run(streamResponse)}>Stream Response</Button>
      <Button onPress={() => request.run(uploadForm)}>Upload Form</Button>
      <Button onPress={() => request.run(replayUpload)}>Replay Upload</Button>
      <Button onPress={() => request.run(echoSocket)}>WebSocket Echo</Button>
      <Button onPress={() => request.run(cancelResponse)}>Cancel Response</Button>
      <Text>{request.status === "success" ? request.data : request.status === "error" ? request.error.message : request.status === "pending" ? "Running..." : "Ready"}</Text>
    </Screen>
  );
}
