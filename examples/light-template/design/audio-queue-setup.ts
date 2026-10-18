import { queueInitialised } from "../data/audio-queue";

export default async function setup() {
  await queueInitialised.set(true);
}
