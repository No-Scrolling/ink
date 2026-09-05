import { createStore } from "@ink/store";

export type Selection = "Option 1" | "Option 2";
export const selection = createStore<Selection>({
  key: "settings.selection",
  version: 1,
  initial: "Option 1",
  decode(value) {
    if (value !== "Option 1" && value !== "Option 2") throw new Error("Invalid selection");
    return value;
  },
});
