import { createElement } from "react";
import { Screen, Stack, Text, type IconAsset } from "./index";
import { forward10Filled, forward30Filled, forward5Filled, pauseFilled, playArrowFilled, replay10Filled, replay30Filled, replay5Filled, skipNextFilled, skipPreviousFilled } from "./icons";

type TransportAction = { seconds?: 5 | 10 | 30; onPress: () => void; onLongPress?: () => void; disabled?: boolean };
export type PlayingScreenProps = {
  image?: string;
  preloadImages?: readonly string[];
  title: string;
  onTitlePress?: () => void;
  artists: readonly { name: string; onPress?: () => void }[];
  playing: boolean;
  loading?: boolean;
  buffering?: boolean;
  onPlayPause: () => void;
  position: number;
  duration: number;
  onSeek?: (position: number) => void;
  previous: TransportAction;
  next: TransportAction;
  actions?: readonly (({ icon: IconAsset; label?: never } | { label: string; icon?: never }) & { selected?: boolean; disabled?: boolean; onPress: () => void })[];
};

function time(milliseconds: number) {
  const seconds = Math.floor(milliseconds / 1000);
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}

const replayIcons = { 5: replay5Filled, 10: replay10Filled, 30: replay30Filled };
const forwardIcons = { 5: forward5Filled, 10: forward10Filled, 30: forward30Filled };

export function PlayingScreen({ image, preloadImages = [], title, onTitlePress, artists, playing, loading = false, buffering = false, onPlayPause, position, duration, onSeek, previous, next, actions = [] }: PlayingScreenProps) {
  if (!Number.isFinite(position) || !Number.isFinite(duration) || position < 0 || duration < 0) {
    throw new Error("Playback times must be finite, non-negative milliseconds");
  }
  const current = Math.min(position, duration);
  const control = (icon: IconAsset, { onPress, onLongPress, disabled }: TransportAction) =>
    createElement("Pressable", { onPress: disabled ? undefined : onPress, onLongPress: disabled ? undefined : onLongPress },
      createElement("Icon", { tight: true, name: icon, size: 56, tone: disabled ? "muted" : "primary" }));
  return createElement(Screen, null,
    createElement("PlayingLayout", { centered: !image },
      createElement(Stack, { gap: 16, align: "stretch" },
        image && createElement(Stack, { align: "center" }, createElement("Image", { src: image, width: 200, height: 200, fit: "cover", retainWhileLoading: true, preload: preloadImages.slice(0, 2) })),
        createElement(Stack, { gap: 0, align: "center" },
          createElement("Pressable", { onPress: onTitlePress }, createElement("PlayingLabel", { size: 22, text: title })),
          artists.map((artist, index) => createElement("Pressable", { key: index, onPress: artist.onPress },
            createElement("PlayingLabel", { size: 14, text: artist.name }))),
        ),
        createElement(Stack, { gap: 1, align: "stretch" },
          createElement("PlayingProgress", { playing: playing && !loading && !buffering, position: current / 1000, duration: duration / 1000, onSeek: onSeek && ((seconds: number) => onSeek(seconds * 1000)) }),
          createElement(Stack, { axis: "horizontal", justify: "space-between" },
            createElement(Text, { size: 12 }, time(current)),
            createElement(Text, { size: 12 }, time(duration))),
        ),
        createElement("PlayingTransport", { loading },
          control(previous.seconds ? replayIcons[previous.seconds] : skipPreviousFilled, previous),
          control(playing ? pauseFilled : playArrowFilled, { onPress: onPlayPause }),
          control(next.seconds ? forwardIcons[next.seconds] : skipNextFilled, next)),
      ),
      createElement("PlayingTransport", null,
        actions.map((action, index) => createElement("Pressable", { key: index, selected: action.selected, onPress: action.disabled ? undefined : action.onPress },
          action.label !== undefined ? createElement(Text, { size: 22, tabularNumbers: true }, action.label) : createElement("Icon", { tight: true, name: action.icon, size: 44, tone: "primary" })))),
    ));
}
