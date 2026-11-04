---
title: "Overlays"
description: "Display an Ink window over other apps."
---

Use `@ink/overlays` to display one opaque, interactive window over other apps. The window has its own React runtime and remains available when the main app screen closes.

## Configure the entry point

Add the module with `ink add @ink/overlays`, then register a file in `ink.toml`:

```toml
[overlays]
entry = "./overlays.tsx"
```

Import every overlay definition from this entry point. Ink bundles the definitions separately from the app's pages.

## Define an overlay

```tsx
// overlays.tsx
import { Canvas, CanvasText, Rectangle } from "ink";
import { defineOverlay } from "@ink/overlays";

function MessageOverlay({ message }: { message: string }) {
  return (
    <Canvas width={360} height={100}>
      <Rectangle x={0} y={0} width={360} height={100} fill="#ffffff" />
      <CanvasText x={12} y={12} width={336} height={36} colour="#000000">
        {message}
      </CanvasText>
      <Rectangle x={12} y={60} width={336} height={28} fill="#000000"
        onPress={() => { void messageOverlay.hide(); }} />
      <CanvasText x={12} y={60} width={336} height={28} colour="#ffffff" align="center">
        Close
      </CanvasText>
    </Canvas>
  );
}

export const messageOverlay = defineOverlay({
  id: "app.message",
  component: MessageOverlay,
});
```

Use a stable, unique ID. Props must be JSON data; the complete show or update request is limited to 8 KiB. Functions and the main screen's React state cannot be passed into the overlay.

## Request access

| Access | Required for |
| --- | --- |
| Android's **Display over other apps** special access | Showing an overlay. |

Request access from the main app screen when the user opens the feature:

```ts
import { overlays } from "@ink/overlays";

if (!await overlays.isAllowed()) {
  await overlays.requestPermission();
}
const allowed = await overlays.isAllowed();
```

`requestPermission()` opens Android settings and completes when the app resumes. Check access again before showing the window. The request does not grant permission itself.

## Show, update and hide

From an app page or action:

```ts
import { messageOverlay } from "../overlays";

await messageOverlay.show({ message: "Download complete" }, {
  width: 360,
  height: 100,
  position: "bottom",
  offset: 12,
});

await messageOverlay.update({ message: "Your files are ready" });
await messageOverlay.hide();
```

Sizes and offsets use Ink layout units. Width defaults to the display width. Height is required. Position is `top`, `center` or `bottom`; it defaults to `bottom`. Windows are horizontally centred. Android applies its system-window insets.

Pass a complete set of window options as the second argument to `update()` to resize or reposition the overlay. Updating the same overlay preserves its local React state. Showing another ID replaces the content. Updating a hidden overlay fails; hiding an overlay that is not visible has no effect.

Read the current window state with `await overlays.getState()`, which returns `{ visible, id }`. Use `useSnapshot(overlays)` to keep a screen updated when the window is shown or closed:

```tsx
import { overlays } from "@ink/overlays";
import { Field, useSnapshot } from "ink";

function OverlayStatus() {
  const state = useSnapshot(overlays);
  return (
    <Field label="Overlay">
      {state.status === "ready" ? (state.data.visible ? "Visible" : "Hidden") : state.status}
    </Field>
  );
}
```

The snapshot also updates when the overlay closes itself, the user closes it from the notification, or access is revoked. Calls to show or update complete when the service accepts the content; rendering follows asynchronously.

## Behaviour and limits

The window accepts touches within its bounds and leaves keyboard focus with the app behind it. Text input, transparency and activity-owned camera, map, video and media controllers are not supported. Use local components rather than routes from the main app.

Overlays can use local React state, bundled images, Store, secure storage, authentication, networking and connectivity. Network and managed-file image sources are not supported in this version.

Ink displays an ongoing Android notification with a **Close overlay** action. Hiding the window releases its renderer and runtime and stops the service. Revoking overlay access also closes it. Android can stop the process; Ink does not automatically restore a dismissed or terminated overlay.

During `ink dev`, hide the overlay before showing it again to load the latest compiled overlay code.

## Try the template

Open **Modules > Overlays** in the template app. Show the overlay, update its message, move it, and open another app. Its counter and close button work independently of the template screen.

<img src="/images/patterns/overlay.png" alt="Ink's interactive overlay displayed over Android settings." width="1080" height="1240" />
