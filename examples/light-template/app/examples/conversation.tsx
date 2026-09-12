import { useRef, useState } from "react";
import { Button, ConversationScreen, Screen, navigate, type ConversationMessage } from "ink";
import wallsocket from "../../assets/images/wallsocket.jpg";

const artwork = { src: wallsocket, width: 180, height: 180 };
const start = new Date(2026, 8, 6, 14, 0).getTime();
const history: ConversationMessage[] = Array.from({ length: 30 }, (_, i) => ({
  id: `history-${i}`, text: `Earlier message ${i + 1}`, timestamp: start + i * 60000, outgoing: i % 3 === 0, author: i % 2 === 0 ? "Alex" : "Sam",
}));
const initial: ConversationMessage[] = [
  { id: "hello", author: "Alex", text: "Have you listened to this album yet?", timestamp: start + 1800000 },
  { id: "album", author: "Alex", text: "Wallsocket by underscores", image: artwork, timestamp: start + 1860000 },
  { id: "reply", text: "Yes! Cops and robbers is my favourite.", outgoing: true, status: "delivered", reply: { author: "Alex", text: "Have you listened to this album yet?" }, timestamp: start + 1920000 },
  { id: "emoji", author: "Sam", text: "❤️", timestamp: start + 1980000 },
  { id: "read", text: "Are you free this evening?", outgoing: true, status: "read", timestamp: start + 2010000 },
  { id: "failed", text: "We should listen to it together.", outgoing: true, status: "failed", timestamp: start + 2040000 },
  { id: "photo", author: "Alex", image: artwork, timestamp: start + 2100000 },
];

export default function Conversation() {
  return <Screen title="Conversation">
    <Button href="/examples/conversation/single">Single chat</Button>
    <Button href="/examples/conversation/group">Group chat</Button>
  </Screen>;
}

export function ConversationExample({ group = false }: { group?: boolean }) {
  const [messages, setMessages] = useState<ConversationMessage[]>(() => initial.map(message => ({ ...message, author: group ? message.author : "Alex" })));
  const [draft, setDraft] = useState("");
  const [olderCount, setOlderCount] = useState(30);
  const nextId = useRef(0);
  function toggleReaction(id: string) {
    setMessages(current => current.map(message => message.id === id ? { ...message, reactions: message.reactions ? undefined : "❤️" } : message));
  }
  function retry(id: string) {
    setMessages(current => current.map(message => message.id === id ? { ...message, status: "sent" as const } : message));
  }
  function send({ text, replyTo }: { text: string; replyTo?: ConversationMessage }) {
    const reply = replyTo ? { author: replyTo.outgoing ? "You" : replyTo.author ?? "Alex", text: replyTo.text || "Photo" } : undefined;
    const message: ConversationMessage = { id: `sent-${nextId.current++}`, text, outgoing: true, status: "sent", timestamp: Date.now(), reply };
    setMessages(current => [...current, message]);
    setDraft("");
  }
  return <ConversationScreen title={group ? "Music club" : "Alex"} group={group} messages={messages} draft={draft} onDraftChange={setDraft} onSend={send}
    actions={message => [{ label: "React ❤️", onPress: () => toggleReaction(message.id) }]}
    onAttach={() => navigate("/actions")}
    onRetry={message => retry(message.id)}
    onDoubleTap={message => toggleReaction(message.id)}
    hasOlder={olderCount > 0}
    onLoadOlder={async () => {
      await new Promise<void>(resolve => setTimeout(resolve, 400));
      const next = Math.max(0, olderCount - 10);
      setMessages(current => [...history.slice(next, olderCount).map(message => ({ ...message, author: group ? message.author : "Alex" })), ...current]);
      setOlderCount(next);
    }} />;
}
