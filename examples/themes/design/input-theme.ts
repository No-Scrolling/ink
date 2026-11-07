import { defineTheme } from "ink/theme";

export default defineTheme({
  text: { size: 48 },
  spacing: { inset: 64, gap: 36 },
  textInput: {
    border: "outline",
    borderWidth: 2,
    borderColour: "muted",
    radius: 14,
    background: "#161616",
    paddingX: 20,
    paddingY: 14,
    fontSize: 44,
    focused: { borderColour: "foreground" },
    invalid: { borderColour: "#ff7575" },
    disabled: { textColour: "muted", borderColour: "#444444" },
    readOnly: { border: "none", background: "#222222" },
  },
});
