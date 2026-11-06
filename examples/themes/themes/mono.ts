import { defineTheme } from "ink/theme";

// Large monospaced type with a built-in switch and tight margins.
export default defineTheme({
  fonts: { body: "../assets/fonts/CommitMono-400-Regular.otf" },
  colours: { background: "#000000", foreground: "#f5f5f5", muted: "#5c5c5c" },
  text: { size: 58 },
  spacing: { inset: 56, gap: 88 },
  header: { layout: "large", size: 76 },
  section: { size: 30 },
  toggle: "switch",
  selection: "check",
});
