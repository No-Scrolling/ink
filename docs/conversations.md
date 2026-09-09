---
title: "Conversations"
description: "Display messages, replies and a composer."
---

## Conversations

`ConversationScreen` owns message presentation, the list, composer and message actions page. Use it in an `app/` page; no extra route or provider is needed for message actions. Supply `ConversationMessage` objects with an `id`, `timestamp` and text or an image. Ink handles keys, rendering and reply previews.

```tsx
<ConversationScreen
  title="Alex"
  messages={messages}
  draft={draft}
  onDraftChange={setDraft}
  actions={message => [{
    label: "React ❤️",
    onPress: () => toggleReaction(message.id),
  }]}
  onSend={({ text, replyTo }) => sendMessage(text, replyTo?.id)}
/>
```

### Reply and send

Long-press a message or image to open its preview and **Reply** action. Reply returns to the chat and scrolls to the bottom. The banner shows the author and text, or “Photo” for an image. Outgoing replies use “You”; incoming replies without an author use the screen title.

`onSend` receives `{ text, replyTo }`, with trimmed text and the selected message. Ink clears the reply after the callback returns. Your app sends the message, saves any reply snapshot and clears its controlled draft.

`actions` supplies additional labels and callbacks. Ink returns to the chat before invoking a callback, so it can update data or navigate elsewhere. Back dismisses the actions page without changing the selected reply or chat scroll position. Optional `onRetry`, `onImagePress` and `onDoubleTap` callbacks receive the message. `Message` remains available independently for custom screens.

### Message details

`timestamp` is milliseconds since the Unix epoch. Ink shows local time for today and adds the month and day for older messages. Set `group` to show incoming authors in a group chat. Outgoing messages omit the author. Reactions appear on the same line.

Set `status` to `sending`, `sent`, `delivered`, `read` or `failed` using your service’s state. Failed messages offer “Tap to try again” when you supply `onRetry`. Images need `src`, `width` and `height`; saved replies need `author` and `text`.

Use `onLoadOlder` and `hasOlder` to prepend history. `onAttach` supplies the composer's plus action. The Single chat and Group chat examples use local data and demonstrate text, image-only messages, replies, reactions and delivery states.

Sending requests keyboard dismissal and the bottom scroll position in the same React update as clearing the reply. The app should add its outgoing message and clear its controlled draft in `onSend`; Ink does not wait for a network acknowledgement. `sending` disables sending, and `loading` shows the initial loading content.
