import { defineTheme } from "ink/theme";

// Warm light theme. Dark mode keeps Ink's inverse; light mode uses paper tones.
export default defineTheme({
  fonts: { body: "../assets/fonts/Inter-Regular.otf", display: "../assets/fonts/Inter-SemiBold.otf" },
  colours: {
    background: "#14120f",
    foreground: "#f3efe6",
    muted: "#857d70",
    light: { background: "#f3efe6", foreground: "#1f1c17", muted: "#8a8378" },
  },
  text: { size: 46 },
  spacing: { inset: 84, gap: 70 },
  header: { layout: "large", size: 80 },
  section: { uppercase: false, size: 34 },
  field: "inline",
  selection: "check",
  toggle: "switch",
});
