import { defineTheme, defineToggle } from "ink/theme";

// Measured from the Atlas keyboard settings mockup, scaled to the 1080 × 1240 frame.
export default defineTheme({
  fonts: { body: "../assets/fonts/Inter-Regular.otf", display: "../assets/fonts/Inter-SemiBold.otf" },
  colours: { background: "#000000", foreground: "#ffffff", muted: "#8e8e93" },
  text: { size: 42 },
  spacing: { inset: 90, gap: 72 },
  header: { layout: "large", size: 70 },
  section: { uppercase: true, size: 29 },
  field: "inline",
  selection: "check",
  toggle: defineToggle({
    width: 128,
    height: 72,
    render: ({ on, disabled, colours }) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 128 72"><g opacity="${disabled ? 0.4 : 1}">
      <rect width="128" height="72" rx="36" fill="${on ? "#6e6e73" : "#3a3a3c"}"/>
      <circle cx="${on ? 92 : 36}" cy="36" r="30" fill="${colours.foreground}"/>
    </g></svg>`,
  }),
});
