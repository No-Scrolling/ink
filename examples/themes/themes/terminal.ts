import { defineTheme } from "ink/theme";

// Green phosphor terminal with monospaced type.
export default defineTheme({
  fonts: { body: "../assets/fonts/JetBrainsMono-Regular.ttf" },
  colours: { background: "#030a04", foreground: "#7cff6b", muted: "#2f7a2a" },
  text: { size: 50 },
  spacing: { inset: 64, gap: 80 },
  header: { size: 46 },
  section: { size: 34 },
});
