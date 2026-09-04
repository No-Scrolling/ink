---
title: "Audio"
description: "Native playback, queues and media controls."
tag: "Design specification"
---

`@ink/audio` owns decoding, buffering, audio focus, routing and media controls. JavaScript sends commands and observes useful state changes; it does not pump audio samples or playback timers.

```tsx
import { Button, Screen, Text, useAction } from "ink";
import { usePlayer } from "@ink/audio";

export function NowPlaying() {
  const player = usePlayer({ session: "main", mode: "detached" });
  const toggle = useAction(() => player.toggle());
  return (
    <Screen title="Now playing">
      <Text>{player.state.current?.title ?? "Nothing playing"}</Text>
      <Button onPress={() => toggle.run()} disabled={!player.state.ready}>
        {player.state.playing ? "Pause" : "Play"}
      </Button>
      {toggle.status === "error" && <Text>{toggle.error.message}</Text>}
    </Screen>
  );
}
```

`usePlayer` attaches after commit and releases its attachment when hidden. Wait for `state.ready` before issuing commands or inspecting a restored queue. Reconnecting observes the live session; mounting the screen does not replace its queue.

## Two lifetimes

An **attached** player is useful for a short voice-note preview. Releasing its last owner stops it. A **detached** session supports music, podcasts and audiobooks: releasing a screen attachment leaves playback under a native media service. `stop()` explicitly stops playback; closing a screen does not.

Detached does not mean immortal. Android can terminate the app process and its service. Persist queue IDs and occasional progress checkpoints to recover, and distinguish restoration from a session that is still live. Headless workers and UI can reconnect by session ID, never by serialising a controller.

## Queue and state

`setQueue(items, { startIndex })`, `play`, `pause`, `seek`, `next`, `previous` and `stop` return promises. Queue items have stable IDs, a supported URL or managed file reference, title and optional artist/artwork metadata. Queue replacement is an explicit user/domain action.

Snapshots include readiness, current item, position, duration, buffering, playing and error. Position is sampled at a useful display rate; smooth native progress and seeking do not need frame-rate JavaScript events. Errors distinguish source, unsupported media and output failures. Skipping a broken item is an app policy, not an automatic consequence of any error.

Native media sessions coordinate hardware keys, lock-screen controls and focus interruptions. State reflects changes made outside the app. Commands issued before readiness reject rather than silently overwriting restored state.

## Providers and offline media

A podcast enclosure or owned audio file can use this player directly. A music service may require a provider SDK, remote-control session, DRM or a dedicated native engine. Its package exposes domain commands and snapshots appropriate to that provider; generic audio does not grant catalogue or offline playback rights.

Use [Downloads](downloads.md) for supported offline files. Resolve a downloaded ID to current content before playback; storage eviction, deletion and expired provider access remain possible.
