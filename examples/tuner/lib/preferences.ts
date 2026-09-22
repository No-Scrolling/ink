import * as v from "valibot";
import { createStore } from "@ink/store";

const preferenceSchema = v.object({
  referenceHz: v.pipe(v.number(), v.finite(), v.minValue(400), v.maxValue(480)),
  flats: v.boolean(),
  showCents: v.optional(v.boolean(), false),
  showFrequency: v.optional(v.boolean(), false),
});

export type Preferences = v.InferOutput<typeof preferenceSchema>;

export const preferences = createStore<Preferences>({
  key: "tuner.preferences",
  version: 1,
  initial: { referenceHz: 440, flats: false, showCents: false, showFrequency: false },
  decode: v.parser(preferenceSchema),
});
