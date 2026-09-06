import { createStore } from "@ink/store";

export const attachments = createStore<string[]>({
  key: "example.attachments", version: 1, initial: [],
  decode(value) {
    if (!Array.isArray(value) || !value.every(id => typeof id === "string")) throw new Error("Saved attachments are invalid");
    return value;
  },
});
