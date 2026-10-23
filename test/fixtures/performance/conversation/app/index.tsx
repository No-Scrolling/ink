import { useEffect, useEffectEvent, useState } from "react";
import { ConversationScreen, Screen, Text, type ConversationMessage } from "ink";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;

function buildIndex(count: number) {
  __inkPost(JSON.stringify({ type: "cpu-start" }));
  const started = performance.now();
  let sum = 0;
  for (let index = 0; index < count; index++) sum += index % 97;
  __inkPost(JSON.stringify({ type: "cpu-done", sum, elapsedMs: performance.now() - started }));
}

export default function App() {
  const [messages, setMessages] = useState<readonly ConversationMessage[] | null>(null);
  const [draft, setDraft] = useState("");
  const [received, setReceived] = useState(0);
  const receive = useEffectEvent((message: Record<string, unknown>) => {
    if (message.command === "populate") setMessages(message.messages as ConversationMessage[]);
    if (message.command === "cpu") buildIndex(Number(message.count));
    if (message.command === "append") {
      const incoming = message.messages as ConversationMessage[];
      setMessages((previous) => [...(previous ?? []), ...incoming].slice(-512));
      setReceived((previous) => previous + incoming.length);
    }
    if (message.command === "prepend")
      setMessages((previous) =>
        message.insert
          ? [message.message as ConversationMessage, ...(previous ?? [])]
          : (previous ?? []).filter((item) => item.id !== "older"),
      );
    if (message.command === "report")
      __inkPost(
        JSON.stringify({
          type: "observed",
          draft,
          received,
          ids: messages?.map((item) => item.id) ?? [],
        }),
      );
  });
  useEffect(() => {
    const stop = onNativeMessage("benchmark", receive);
    __inkPost(JSON.stringify({ type: "ready" }));
    return stop;
  }, []);
  if (!messages)
    return (
      <Screen title="Conversation">
        <Text>Loading conversation</Text>
      </Screen>
    );
  return (
    <ConversationScreen
      title="Conversation"
      group
      messages={messages}
      draft={draft}
      onDraftChange={setDraft}
      onSend={({ text }) => {
        setMessages((previous) => [
          ...(previous ?? []),
          {
            id: "outgoing",
            text,
            timestamp: 1700000000000,
            outgoing: true,
            status: "sending",
          },
        ]);
        setDraft("");
      }}
    />
  );
}
