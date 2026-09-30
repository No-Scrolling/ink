import { useEffect, useEffectEvent, useState } from "react";
import { ConversationScreen, Screen, Text, type ConversationMessage } from "ink";
import { onNativeMessage } from "ink/native";

declare const __inkPost: (source: string) => void;

export default function App() {
  const [messages, setMessages] = useState<readonly ConversationMessage[] | null>(null);
  const [draft, setDraft] = useState("");
  const [received, setReceived] = useState(0);
  const receive = useEffectEvent((message: Record<string, unknown>) => {
    if (message.command === "populate") setMessages(message.messages as ConversationMessage[]);
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
