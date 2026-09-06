import { createElement, useRef, useState } from "react";
import icons from "./conversation.ink-icons";
import { Button, Image, List, Screen, Stack, Text, TextInput } from "./index";
import { back, presentPage } from "./navigation";

export type ReplyPreview = { author: string; text: string; onPress?: () => void };
export type MessageProps = {
  text?: string;
  timestamp: number;
  outgoing?: boolean;
  author?: string;
  image?: { src: string; width: number; height: number; onPress?: () => void };
  reply?: ReplyPreview;
  reactions?: string;
  status?: "sending" | "sent" | "delivered" | "read" | "failed";
  onRetry?: () => void;
  onLongPress?: () => void;
  onDoubleTap?: () => void;
};

function messageTime(timestamp: number) {
  const date = new Date(timestamp);
  const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const day = `${months[date.getMonth()]} ${date.getDate()}`;
  const time = `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
  return date.toDateString() === new Date().toDateString() ? time : `${day} ${time}`;
}

function Reply({ author, text, onPress }: ReplyPreview) {
  return createElement("PlayingPressable", { onPress },
    createElement("MessageQuote", null,
      createElement(Stack, { gap: 3 },
        createElement(Text, { size: 14 }, `Replying to ${author}`),
        createElement(Text, { size: 14, maxLines: 1 }, text))));
}

export function Message({ text, timestamp, outgoing = false, author, image, reply, reactions, status, onRetry, onLongPress, onDoubleTap }: MessageProps) {
  const lastTap = useRef(0);
  const delivery = outgoing && (status === "sent" ? "Sent" : status === "delivered" ? "Delivered" : status === "read" ? "Read" : undefined);
  const label = status === "failed" ? (onRetry ? "Could not send. Tap to try again" : "Could not send")
    : status === "sending" ? "Sending…" : [!outgoing && author, messageTime(timestamp), delivery].filter(Boolean).join(", ");
  const metadata = createElement(Stack, { axis: "horizontal", align: "center", gap: 0 },
    createElement(Text, { size: 14 }, label),
    reactions && createElement(Text, { size: 12 }, `, ${reactions}`));
  return createElement("Message", { outgoing },
    createElement<{ onLongPress?: () => void; onPress: () => void }>("PlayingPressable", {
      onLongPress,
      onPress() {
        const now = Date.now();
        if (lastTap.current && now - lastTap.current < 300) { lastTap.current = 0; onDoubleTap?.(); }
        else lastTap.current = now;
      },
    }, createElement(Stack, { gap: 4, align: "stretch" },
      createElement(Stack, { align: outgoing ? "end" : "start" },
        status === "failed" && onRetry ? createElement("PlayingPressable", { onPress: onRetry }, metadata) : metadata),
      createElement(Stack, { gap: 8, align: "stretch" },
        reply && createElement(Reply, reply),
        image && createElement("PlayingPressable", { onPress: image.onPress, onLongPress },
          createElement(Image, { src: image.src, width: image.width, height: image.height, fit: "contain" })),
        text && createElement(Text, { size: 20 }, text),
      ),
    )));
}

export type MessageAction = { label: string; onPress: () => void };

export type ConversationMessage = Pick<MessageProps, "text" | "timestamp" | "outgoing" | "author" | "reactions" | "status"> & {
  id: string;
  image?: { src: string; width: number; height: number };
  reply?: { author: string; text: string };
};

export type ConversationScreenProps<T extends ConversationMessage = ConversationMessage> = {
  title: string;
  group?: boolean;
  messages: readonly T[];
  draft: string;
  onDraftChange: (draft: string) => void;
  onSend: (message: { text: string; replyTo?: T }) => void;
  onAttach?: () => void;
  onRetry?: (message: T) => void;
  onImagePress?: (message: T) => void;
  onDoubleTap?: (message: T) => void;
  actions?: (message: T) => readonly MessageAction[];
  onLoadOlder?: () => Promise<void>;
  hasOlder?: boolean;
  loading?: boolean;
  sending?: boolean;
};

export function ConversationScreen<T extends ConversationMessage>({ title, group = false, messages, draft, onDraftChange, onSend, onAttach, onRetry, onImagePress, onDoubleTap, actions, onLoadOlder, hasOlder = false, loading = false, sending = false }: ConversationScreenProps<T>) {
  const [reply, setReply] = useState<T>();
  const [scrollToEnd, setScrollToEnd] = useState(0);
  const text = draft.trim();
  const send = () => {
    if (!text || sending) return;
    onSend({ text, replyTo: reply });
    setReply(undefined);
  };
  function replyPreview(message: T): ReplyPreview {
    return { author: message.outgoing ? "You" : message.author ?? title, text: message.text || "Photo" };
  }
  function renderMessage(message: T, onLongPress?: () => void) {
    return createElement(Message, {
      text: message.text, timestamp: message.timestamp, outgoing: message.outgoing,
      author: group ? message.author : undefined, reply: message.reply,
      reactions: message.reactions, status: message.status,
      image: message.image && { ...message.image, onPress: onImagePress && (() => onImagePress(message)) },
      onRetry: onRetry && (() => onRetry(message)),
      onDoubleTap: onDoubleTap && (() => onDoubleTap(message)),
      onLongPress,
    });
  }
  function messageItem(message: T) {
    const open = () => {
      presentPage(createElement(Screen, { title: "Message actions" },
        renderMessage(message),
        createElement(Button, { onPress: () => {
          setReply(message);
          setScrollToEnd(request => request + 1);
          back();
        } }, "Reply"),
        actions?.(message).map((action, actionIndex) => createElement(Button, {
          key: actionIndex,
          onPress: () => { back(); action.onPress(); },
        }, action.label))));
    };
    return renderMessage(message, open);
  }
  const iconButton = (name: string, onPress?: () => void) => createElement("PlayingPressable", { onPress },
    createElement("Icon", { name, size: 28, filled: true, tone: onPress ? "primary" : "muted" }));
  return createElement("Screen", { title, pinnedFooter: true, initialEnd: true, scrollToEnd },
    loading ? createElement(Text, { size: 18, align: "center" }, "Loading…")
      : createElement(List<T>, { items: messages, keyExtractor: message => message.id, renderItem: messageItem, gap: 28, followEnd: true, initialEnd: true, onLoadOlder, hasOlder }),
    !loading && messages.length === 0 && createElement(Text, { size: 18, align: "center" }, "No messages yet"),
    createElement(Stack, { gap: 8, align: "stretch" },
      reply && createElement("ConversationComposer", null,
        createElement(Reply, replyPreview(reply)), iconButton(icons.close, () => setReply(undefined))),
      createElement("ConversationComposer", null,
        onAttach ? iconButton(icons.add, onAttach) : createElement(Stack),
        createElement(TextInput, { value: draft, onChange: onDraftChange, onSubmit: send, placeholder: "Message…", action: "done" }),
        iconButton(icons.send, text && !sending ? send : undefined)),
    ));
}
