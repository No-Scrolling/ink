import { defineTheme, defineToggle } from "ink/theme";

// Measured from the compact Atlas settings mockup at 1080 / 299 ≈ 3.61× its panel width.
export default defineTheme({
  fonts: { body: "../assets/fonts/Inter-Regular.otf", display: "../assets/fonts/Inter-Medium.otf" },
  colours: { background: "#000000", foreground: "#ffffff", muted: "#7f7f84" },
  text: { size: 45 },
  spacing: { inset: 92, gap: 69 },
  header: { layout: "large", size: 74, top: 51, bottom: 51, cutout: "below", back: { position: "end", icon: "arrow" }, iconSize: 60 },
  section: { uppercase: true, size: 31, gap: 48 },
  field: "inline",
  variants: {
    // A calmer reading screen: warm colours, monospaced type and larger text.
    reader: {
      fonts: { body: "../assets/fonts/CommitMono-400-Regular.otf", display: "../assets/fonts/CommitMono-400-Regular.otf" },
      colours: { background: "#14110d", foreground: "#efe6d6", muted: "#9c907c" },
      text: { size: 50 },
      header: { size: 64 },
    },
  },
  selection: "check",
  toggle: defineToggle({
    width: 130,
    height: 76,
    render: ({ on, disabled, colours }) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 130 76"><g opacity="${disabled ? 0.4 : 1}">
      <rect width="130" height="76" rx="38" fill="${on ? "#8e8e93" : "#3a3a3c"}"/>
      <circle cx="${on ? 92 : 38}" cy="38" r="33" fill="${on ? "#f2f2f4" : colours.foreground}"/>
    </g></svg>`,
  }),
});
