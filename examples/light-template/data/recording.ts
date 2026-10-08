import * as v from "valibot";
import { createStore } from "@ink/store";

export const savedRecording = createStore<string | null>({
  key: "template.recording",
  version: 1,
  initial: null,
  decode: v.parser(v.nullable(v.string())),
});
