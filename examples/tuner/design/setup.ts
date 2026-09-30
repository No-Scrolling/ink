import { preferences } from "../lib/preferences";

export default async function setup() {
  await preferences.set({ referenceHz: 442, flats: false, showCents: true, showFrequency: true });
}
