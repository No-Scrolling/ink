import * as v from "valibot";
import { createStore } from "@ink/store";

export const attachments = createStore<string[]>({
  key: "example.attachments",
  version: 1,
  initial: [],
  decode: v.parser(v.array(v.string())),
});
