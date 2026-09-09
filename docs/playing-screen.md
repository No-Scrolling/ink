---
title: "Playing screen"
description: "Build a player with consistent controls."
---

## Playing screen

`PlayingScreen` groups optional artwork, title/artists, progress and transport controls at the top of the content area below the header. Its `actions` stay at the bottom and are distributed evenly. Omitting `image` centres the main group vertically between the header and bottom actions; a loading image reserves its space.

Pass playback state and callbacks:

| Props | Purpose |
| --- | --- |
| `playing`, `onPlayPause` | Control playback. |
| `position`, `duration`, optional `onSeek` | Show progress in milliseconds and handle seeking. |
| `title`, optional `onTitlePress` | Display an actionable title. |
| `artists` | Names with optional `onPress` callbacks. |
| `previous`, `next` | Required actions with `onPress`, optional `onLongPress` and `disabled`. Set `seconds` to 5, 10 or 30 for seek icons. |
| `actions` | Bottom controls with `icon`, `onPress`, optional `selected` and `disabled`. |

Your app controls playback and queues. The template’s **Image** and **No Image** examples simulate progress without playing audio.

A long press calls `onLongPress` once and suppresses the tap on release. Moving away cancels it. Bottom actions can change icons and use `selected` for an underline.

