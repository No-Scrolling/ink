import { defineTheme, defineToggle } from "ink/theme";

// Sets every theme key, so preparing this fixture checks that the TypeScript types, the build-time
// checker and the Rust decoder agree.
export default defineTheme({
  fonts: {
    body: "../../../../examples/themes/assets/fonts/Inter-Regular.otf",
    display: "../../../../examples/themes/assets/fonts/Inter-Medium.otf",
  },
  colours: {
    background: "#000000",
    foreground: "#c8ff00",
    muted: "#7f7f84",
    light: { background: "#ffffff", foreground: "#202020", muted: "#808080" },
  },
  text: { size: 45 },
  spacing: { inset: 90, gap: 70 },
  header: {
    layout: "large",
    size: 74,
    top: 50,
    bottom: 50,
    back: { position: "end", icon: "arrow" },
    iconSize: 60,
    cutout: "below",
  },
  section: { uppercase: true, size: 31, gap: 20 },
  subtitle: { size: 30, tone: "muted" },
  scrollBar: { colour: "#ff6600", visibility: "scrolling", track: false, width: 8, inset: 30 },
  field: "inline",
  selection: "check",
  variants: {
    reader: {
      fonts: { body: "../../../../examples/themes/assets/fonts/JetBrainsMono-Regular.ttf" },
      colours: { background: "#101820" },
      text: { size: 60 },
      header: { layout: "centred" },
    },
  },
  toggle: defineToggle({
    width: 120,
    height: 60,
    render: ({ on, disabled }) =>
      `<svg viewBox="0 0 120 60"><rect width="120" height="60" fill="${disabled ? "#333333" : on ? "#ffffff" : "#888888"}"/></svg>`,
  }),
});
