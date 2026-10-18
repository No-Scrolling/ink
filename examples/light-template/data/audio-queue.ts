import * as v from "valibot";
import { createStore } from "@ink/store";
import type { AudioItem } from "@ink/audio";
import track from "../assets/audio/cant_help.mp3";

export const queueSession = "queue-detached";
export const demoEpisodes: AudioItem[] = ["A", "B", "C", "D", "E"].map((letter) => ({
  id: letter,
  src: track,
  title: `Episode ${letter}`,
}));

export const queueInitialised = createStore<boolean>({
  key: "template.audio-queue.initialised",
  version: 1,
  initial: false,
  decode: v.parser(v.boolean()),
});
