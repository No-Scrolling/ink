import { createElement, useRef } from "./react";
import { Screen, ErrorState, type IconAsset } from "./index";
import { useAction } from "./action";
import { playbackController } from "./playback-binding";
import { forward10Filled, forward30Filled, forward5Filled, pauseFilled, playArrowFilled, replay10Filled, replay30Filled, replay5Filled, skipNextFilled, skipPreviousFilled } from "./icons";

type PlaybackAction = () => void | Promise<void>;
export interface Playback {
  readonly state: { position: number; duration: number; playWhenReady: boolean; loading?: boolean; buffering?: boolean; speed?: number };
  readonly [playbackController]?: number;
  toggle(): void | Promise<void>;
  seek(position: number): void | Promise<void>;
  seekBy?(offset: number): void | Promise<void>;
  previous?(): void | Promise<void>;
  next?(): void | Promise<void>;
}

export type TransportControl = { onLongPress?: PlaybackAction; disabled?: boolean } & (
  | { kind: "track"; seconds?: never; icon?: never; onPress?: never }
  | { kind: "skip"; seconds: 5 | 10 | 30; icon?: never; onPress?: never }
  | { kind: "custom"; icon: IconAsset; onPress: PlaybackAction; seconds?: never }
);
export type PlayingScreenProps = {
  image?: string;
  preloadImages?: readonly string[];
  title: string;
  onTitlePress?: () => void;
  artists: readonly { name: string; onPress?: () => void }[];
  playback: Playback;
  previous?: TransportControl;
  next?: TransportControl;
  actions?: readonly (({ icon: IconAsset; label?: never } | { label: string; icon?: never }) & { selected?: boolean; disabled?: boolean; onPress: PlaybackAction })[];
};

const replayIcons = { 5: replay5Filled, 10: replay10Filled, 30: replay30Filled };
const forwardIcons = { 5: forward5Filled, 10: forward10Filled, 30: forward30Filled };

export function PlayingScreen({ image, preloadImages = [], title, onTitlePress, artists, playback, previous = { kind: "track" }, next = { kind: "track" }, actions = [] }: PlayingScreenProps) {
  const retry = useRef<PlaybackAction>(() => {});
  const command = useAction(async (operation: PlaybackAction) => {
    retry.current = operation;
    await operation();
  });
  const run = command.run;
  const transport = (action: TransportControl, direction: -1 | 1): PlaybackAction | undefined => {
    if (action.kind === "custom") return action.onPress;
    if (action.kind === "skip") {
      const seekBy = playback.seekBy;
      return seekBy && (() => seekBy.call(playback, direction * action.seconds * 1000));
    }
    const move = direction === -1 ? playback.previous : playback.next;
    return move && (() => move.call(playback));
  };
  const previousPress = transport(previous, -1);
  const nextPress = transport(next, 1);
  const bind = (operation: PlaybackAction | undefined) => operation && (() => run(operation));
  const { position, duration, playWhenReady: playing, loading = false, buffering = false, speed = 1 } = playback.state;
  const clock = playback[playbackController];
  if (command.status === "error") return createElement(Screen, { title: "Playback" },
    createElement(ErrorState, { message: command.error.message, onRetry: () => run(retry.current) }));
  if (!Number.isFinite(position) || !Number.isFinite(duration) || position < 0 || duration < 0) {
    throw new Error("Playback times must be finite, non-negative milliseconds");
  }
  return createElement("PlayingScreen", {
    image, preloadImages, title, onTitlePress, playing, loading, buffering, position, duration, clock, speed,
    artists: artists.map(artist => ({ name: artist.name, interactive: !!artist.onPress })),
    onArtistPress: (index: number) => artists[index]?.onPress?.(),
    onPlayPause: () => run(() => playback.toggle()), onSeek: (seconds: number) => run(() => playback.seek(seconds * 1000)),
    previousIcon: previous.kind === "custom" ? previous.icon : previous.kind === "skip" ? replayIcons[previous.seconds] : skipPreviousFilled,
    previousDisabled: previous.disabled || !previousPress,
    onPreviousPress: bind(previousPress), onPreviousLongPress: bind(previous.onLongPress),
    nextIcon: next.kind === "custom" ? next.icon : next.kind === "skip" ? forwardIcons[next.seconds] : skipNextFilled,
    nextDisabled: next.disabled || !nextPress,
    onNextPress: bind(nextPress), onNextLongPress: bind(next.onLongPress),
    playIcon: playArrowFilled, pauseIcon: pauseFilled,
    actions: actions.map(({ onPress, ...action }) => action),
    onActionPress: (index: number) => { const action = actions[index]; if (action) run(action.onPress); },
  });
}
