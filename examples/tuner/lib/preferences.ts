import { createStore } from "@ink/store";

export interface Preferences {
  referenceHz: number;
  flats: boolean;
  showCents: boolean;
  showFrequency: boolean;
}

export const preferences = createStore<Preferences>({
  key: "tuner.preferences",
  version: 1,
  initial: { referenceHz: 440, flats: false, showCents: false, showFrequency: false },
  decode(value) {
    if (typeof value !== "object" || value === null
      || !("referenceHz" in value) || typeof value.referenceHz !== "number"
      || !Number.isFinite(value.referenceHz) || value.referenceHz < 400 || value.referenceHz > 480
      || !("flats" in value) || typeof value.flats !== "boolean"
      || ("showCents" in value && typeof value.showCents !== "boolean")
      || ("showFrequency" in value && typeof value.showFrequency !== "boolean")) {
      throw new Error("Could not read tuner settings.");
    }
    return {
      referenceHz: value.referenceHz,
      flats: value.flats,
      showCents: "showCents" in value && value.showCents === true,
      showFrequency: "showFrequency" in value && value.showFrequency === true,
    };
  },
});
