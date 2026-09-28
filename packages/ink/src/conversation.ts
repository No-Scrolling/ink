import { createElement, useEffect, useRef, useState, type ComponentProps } from "./react";
import { addFilled, closeFilled, sendFilled } from "./icons";
import { Button, Image, List, Screen, Stack, Text, TextInput } from "./index";
import { back, presentPage } from "./navigation";
import { textLinks } from "./links";
import type { LinkPreviewData } from "./link-preview";
import tick from "./icons/conversation/tick.svg";
import tickDouble from "./icons/conversation/tick-double.svg";
import view from "./icons/conversation/view.svg";
import { callNative } from "./native";
import { openLink } from "./external";
import { nativeListRow, nativeListTemplate } from "./native-list";

export type ReplyPreview = { author: string; text: string; onPress?: () => void };
export type MessageProps = {
  text?: string;
  onLinkPress?: (url: string) => void;
  linkPreview?: LinkPreviewData;
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

const statusIcons = { sending: tick, sent: tick, delivered: tickDouble, read: view };

function messageTime(timestamp: number) {
  const date = new Date(timestamp);
  const months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const day = `${months[date.getMonth()]} ${date.getDate()}`;
  const time = `${String(date.getHours()).padStart(2, "0")}:${String(date.getMinutes()).padStart(2, "0")}`;
  return date.toDateString() === new Date().toDateString() ? time : `${day} ${time}`;
}

function Reply({ author, text, onPress }: ReplyPreview) {
  return createElement("Pressable", { onPress },
    createElement("MessageQuote", null,
      createElement(Stack, { gap: 3 },
        createElement(Text, { size: 14 }, `Replying to ${author}`),
        createElement(Text, { size: 14, maxLines: 1 }, text))));
}

const messageTemplate = /* @__PURE__ */ nativeListTemplate((node, field) => node("MessageContent", Object.fromEntries([
  "outgoing", "label", "reactions", "statusIcon", "statusMuted", "reply", "image", "preview", "parts",
  "onPress", "onLongPress", "onRetry", "onImagePress", "onReplyPress", "onLinkPress", "onPreviewPress",
].map(name => [name, field(name)]))));

function messageFields({ text, onLinkPress = openLink, linkPreview, timestamp, outgoing = false, author, image, reply,
  reactions, status, onRetry, onLongPress }: MessageProps, onPress: () => void) {
  const displayText = text && linkPreview
    ? textLinks(text).filter(link => link.url === linkPreview.url).reverse()
      .reduce((value, link) => value.slice(0, link.start) + value.slice(link.end), text).trim()
    : text;
  const parts: { text: string; url?: string }[] = [];
  let offset = 0;
  if (displayText) {
    for (const link of textLinks(displayText)) {
      parts.push({ text: displayText.slice(offset, link.start) });
      parts.push({ text: displayText.slice(link.start, link.end), url: link.url });
      offset = link.end;
    }
    parts.push({ text: displayText.slice(offset) });
  }
  const failed = outgoing && status === "failed";
  return { outgoing, label: failed ? (onRetry ? "Tap to retry" : "Could not send")
      : [!outgoing && author, messageTime(timestamp)].filter(Boolean).join(", "),
    reactions, statusIcon: outgoing && status && status !== "failed" ? statusIcons[status] : undefined,
    statusMuted: status === "sending", parts,
    reply: reply && { author: reply.author, text: reply.text },
    image: image && { src: image.src, width: image.width, height: image.height },
    preview: linkPreview && { ...linkPreview, domain: linkPreview.url.replace(/^https?:\/\/(?:www\.)?/, "").split(/[/?#]/, 1)[0] },
    onPress, onLongPress, onRetry: failed ? onRetry : undefined, onImagePress: image?.onPress,
    onReplyPress: reply?.onPress, onLinkPress, onPreviewPress: linkPreview && (() => onLinkPress(linkPreview.url)),
  };
}

function doubleTap(previous: { current: number }, callback?: () => void) {
  const now = Date.now();
  if (previous.current && now - previous.current < 300) {
    previous.current = 0;
    if (callback) { void callNative("interaction", "haptic", null); callback(); }
  } else previous.current = now;
}

export function Message(props: MessageProps) {
  const lastTap = useRef(0);
  return createElement("MessageContent", messageFields(props, () => doubleTap(lastTap, props.onDoubleTap)));
}

export type MessageAction = { label: string; onPress: () => void };

export type ConversationMessage = Pick<MessageProps, "text" | "timestamp" | "outgoing" | "author" | "reactions" | "status" | "linkPreview"> & {
  id: string;
  image?: { src: string; width: number; height: number };
  reply?: { author: string; text: string };
};

export type ConversationScreenProps<T extends ConversationMessage = ConversationMessage> = {
  title: string;
  rightAction?: ComponentProps<typeof Screen>["rightAction"];
  group?: boolean;
  messages: readonly T[];
  draft: string;
  onDraftChange: (draft: string) => void;
  onSend: (message: { text: string; replyTo?: T }) => void;
  onAttach?: () => void;
  onRetry?: (message: T) => void;
  onImagePress?: (message: T) => void;
  onLinkPress?: (url: string) => void;
  onDoubleTap?: (message: T) => void;
  actions?: (message: T) => readonly MessageAction[];
  onLoadOlder?: () => Promise<void>;
  hasOlder?: boolean;
  loading?: boolean;
  sending?: boolean;
};

export function ConversationScreen<T extends ConversationMessage>({ title, rightAction, group = false, messages, draft, onDraftChange, onSend, onAttach, onRetry, onImagePress, onLinkPress, onDoubleTap, actions, onLoadOlder, hasOlder = false, loading = false, sending = false }: ConversationScreenProps<T>) {
  const [reply, setReply] = useState<T>();
  const [scrollToEnd, setScrollToEnd] = useState(0);
  const taps = useRef(new Map<string, { current: number }>());
  const previousMessages = useRef(new Set(messages.map(message => message.id)));
  useEffect(() => {
    const addedOutgoing = messages.some(message => message.outgoing && message.status === "sending"
      && !previousMessages.current.has(message.id));
    previousMessages.current = new Set(messages.map(message => message.id));
    for (const key of taps.current.keys()) if (!previousMessages.current.has(key)) taps.current.delete(key);
    if (addedOutgoing) setScrollToEnd(request => request + 1);
  }, [messages]);
  const text = draft.trim();
  const send = () => {
    if (!text || sending) return;
    onSend({ text, replyTo: reply });
    setReply(undefined);
    setScrollToEnd(request => request + 1);
  };
  function replyPreview(message: T): ReplyPreview {
    return { author: message.outgoing ? "You" : message.author ?? title, text: message.text || "Photo" };
  }
  function messageProps(message: T, onLongPress?: () => void): MessageProps {
    const image = message.image;
    return {
      text: message.text, timestamp: message.timestamp, outgoing: message.outgoing,
      onLinkPress, linkPreview: message.linkPreview,
      author: group ? message.author : undefined, reply: message.reply,
      reactions: message.reactions, status: message.status,
      image: image && { ...image, onPress: () => {
        if (onImagePress) onImagePress(message);
        else presentPage(createElement(Screen, { title: "Photo", centered: true },
          createElement(Image, { ...image, fit: "contain", bleed: true, zoomable: true })));
      } },
      onRetry: onRetry && (() => onRetry(message)),
      onDoubleTap: onDoubleTap && (() => onDoubleTap(message)),
      onLongPress,
    };
  }
  function renderMessage(message: T, onLongPress?: () => void) { return createElement(Message, messageProps(message, onLongPress)); }
  function openMessage(message: T) {
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
  }
  const messageItem = nativeListRow((message: T) => renderMessage(message, () => openMessage(message)), message => {
    const props = messageProps(message, () => openMessage(message));
    return [messageFields(props, () => {
      let previous = taps.current.get(message.id);
      if (!previous) { previous = { current: 0 }; taps.current.set(message.id, previous); }
      doubleTap(previous, props.onDoubleTap);
    })];
  }, messageTemplate, message => [message.text, message.timestamp, messageTime(message.timestamp), message.outgoing,
    group ? message.author : undefined, message.reactions, message.status, !!onRetry,
    message.image?.src, message.image?.width, message.image?.height,
    message.reply?.author, message.reply?.text, message.linkPreview?.url, message.linkPreview?.title,
    message.linkPreview?.icon, message.linkPreview?.image?.src, message.linkPreview?.image?.width, message.linkPreview?.image?.height]);
  const iconButton = (name: string, onPress?: () => void) => createElement("Pressable", { onPress },
    createElement("Icon", { name, size: 28, tone: onPress ? "primary" : "muted" }));
  return createElement("Screen", { title, rightIcon: rightAction?.icon, onRightPress: rightAction?.onPress, pinnedFooter: true, initialEnd: true, scrollToEnd },
    loading ? createElement(Text, { size: 18, align: "center" }, "Loading…")
      : createElement(List<T>, { items: messages, keyExtractor: message => message.id, renderItem: messageItem, gap: 28, followEnd: true, initialEnd: true, onLoadOlder, hasOlder }),
    !loading && messages.length === 0 && createElement(Text, { size: 18, align: "center" }, "No messages yet"),
    createElement(Stack, { gap: 8, align: "stretch" },
      reply && createElement("ConversationComposer", null,
        createElement(Reply, replyPreview(reply)), iconButton(closeFilled, () => setReply(undefined))),
      createElement("ConversationComposer", null,
        onAttach ? iconButton(addFilled, onAttach) : createElement(Stack),
        createElement(TextInput, { value: draft, onChange: onDraftChange, placeholder: "Message…", action: "return" }),
        iconButton(sendFilled, text && !sending ? send : undefined)),
    ));
}
